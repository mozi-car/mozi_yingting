use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode, ThreadsafeCallContext};
use napi_derive::napi;
use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Clone)]
struct Endpoint {
    socket: Arc<UdpSocket>,
    target: SocketAddr,
    sd_target: SocketAddr,
    reliable: bool,
}
struct Periodic {
    stop: Arc<std::sync::atomic::AtomicBool>,
    worker: Option<JoinHandle<()>>,
    period: Duration,
}
struct ServiceOffer {
    service: u16,
    instance: u16,
    major: u8,
    minor: u32,
    ttl: u32,
}
struct Subscription {
    service: u16,
    instance: u16,
    eventgroup: u16,
    ttl: u32,
}
struct AppState {
    endpoint: Option<Endpoint>,
    offers: HashMap<(u16, u16), ServiceOffer>,
    subscriptions: HashMap<String, Subscription>,
    running: Arc<std::sync::atomic::AtomicBool>,
    worker: Option<JoinHandle<()>>,
    periodic: HashMap<String, Periodic>,
    handler_ids: HashMap<String, HandlerRegistration>,
}
#[derive(Clone)]
enum HandlerRegistration {
    Trace,
    State,
    Message {
        service: u16,
        instance: u16,
        method: u16,
    },
    Availability {
        service: u16,
        instance: u16,
    },
}
#[napi(object)]
#[derive(Clone)]
pub struct CallbackData {
    pub service: Option<u16>,
    pub instance: Option<u16>,
    pub method: Option<u16>,
    pub available: Option<bool>,
    pub subscription_id: Option<String>,
    pub status: Option<i32>,
    pub text: Option<String>,
    pub message: Option<ReceivedMessage>,
}
#[napi(object)]
#[derive(Clone)]
pub struct CallbackEvent {
    #[napi(js_name = "type")]
    pub kind: String,
    pub data: CallbackData,
}
type Callback = ThreadsafeFunction<
    CallbackEvent,
    Unknown<'static>,
    (String, CallbackData),
    Status,
    false,
>;
struct CallbackRegistration {
    app: String,
    callback: Callback,
}
#[napi(object)]
#[derive(Clone)]
pub struct ReceivedMessage {
    pub service: u16,
    pub method: u16,
    pub length: u32,
    pub client: u16,
    pub session: u16,
    #[napi(js_name = "protocolVersion")]
    pub protocol_version: u8,
    #[napi(js_name = "interfaceVersion")]
    pub interface_version: u8,
    #[napi(js_name = "messageType")]
    pub message_type: u8,
    #[napi(js_name = "returnCode")]
    pub return_code: u8,
    pub payload: Vec<u8>,
}
static APPS: OnceLock<Mutex<HashMap<String, Arc<Mutex<AppState>>>>> = OnceLock::new();
static CALLBACKS: OnceLock<Mutex<HashMap<String, CallbackRegistration>>> = OnceLock::new();
fn apps() -> &'static Mutex<HashMap<String, Arc<Mutex<AppState>>>> {
    APPS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn callbacks() -> &'static Mutex<HashMap<String, CallbackRegistration>> {
    CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn emit_event(app: &str, kind: &str, data: CallbackData) {
    if let Ok(map) = callbacks().lock() {
        for registration in map.values().filter(|r| r.app == app) {
            let _ = registration.callback.call(CallbackEvent { kind: kind.into(), data: data.clone() }, ThreadsafeFunctionCallMode::NonBlocking);
        }
    }
}
fn state(name: &str) -> napi::Result<Arc<Mutex<AppState>>> {
    apps()
        .lock()
        .map_err(|_| Error::from_reason("vSomeIP state poisoned"))?
        .get(name)
        .cloned()
        .ok_or_else(|| Error::from_reason("application not found"))
}

fn encode(m: &SomeipMessage, payload: &[u8]) -> Vec<u8> {
    // SOME/IP: service, method, length, client, session, protocol, interface, type, return.
    let length = (8 + payload.len()) as u32;
    let mut b = Vec::with_capacity(16 + payload.len());
    b.extend_from_slice(&m.service.to_be_bytes());
    b.extend_from_slice(&m.method.to_be_bytes());
    b.extend_from_slice(&length.to_be_bytes());
    b.extend_from_slice(&m.client.to_be_bytes());
    b.extend_from_slice(&m.session.to_be_bytes());
    b.extend_from_slice(&m.protocol_version.to_be_bytes());
    b.extend_from_slice(&m.interface_version.to_be_bytes());
    b.push(m.message_type);
    b.push(m.return_code);
    b.extend_from_slice(payload);
    b
}
fn sd_packet(entry: [u8; 16], reboot: bool) -> Vec<u8> {
    // SOME/IP-SD payload: flags/reserved, length of entries, entries, options length.
    let mut payload = vec![if reboot { 0x80 } else { 0 }, 0, 0, 0, 0, 0, 0, 16];
    payload.extend_from_slice(&entry);
    payload.extend_from_slice(&[0, 0, 0, 0]);
    let mut packet = Vec::with_capacity(16 + payload.len());
    packet.extend_from_slice(&0xffffu16.to_be_bytes());
    packet.extend_from_slice(&0x8100u16.to_be_bytes());
    packet.extend_from_slice(&((payload.len() + 8) as u32).to_be_bytes());
    packet.extend_from_slice(&[0, 0, 0, 0, 1, 0, 0, 0]);
    packet.extend_from_slice(&payload);
    packet
}
fn send_sd(ep: &Endpoint, entry: [u8; 16]) -> Result<()> {
    ep.socket
        .send_to(&sd_packet(entry, true), ep.sd_target)
        .map_err(|e| Error::from_reason(format!("SOME/IP-SD send to {}: {e}", ep.sd_target)))?;
    Ok(())
}
fn sd_entry(
    kind: u8,
    service: u16,
    instance: u16,
    major: u8,
    minor: u32,
    ttl: u32,
    eventgroup: u16,
) -> [u8; 16] {
    let mut e = [0u8; 16];
    e[0] = kind;
    e[1] = 0;
    e[2..4].copy_from_slice(&service.to_be_bytes());
    e[4..6].copy_from_slice(&instance.to_be_bytes());
    e[6] = major;
    e[7..10].copy_from_slice(&ttl.to_be_bytes()[1..]);
    e[10..14].copy_from_slice(&minor.to_be_bytes());
    e[14..16].copy_from_slice(&eventgroup.to_be_bytes());
    e
}
fn decode(b: &[u8]) -> Option<ReceivedMessage> {
    if b.len() < 16 {
        return None;
    }
    let u16v = |i| u16::from_be_bytes([b[i], b[i + 1]]);
    let length = u32::from_be_bytes([b[4], b[5], b[6], b[7]]);
    if length < 8 || length as usize + 8 > b.len() {
        return None;
    }
    Some(ReceivedMessage {
        service: u16v(0),
        method: u16v(2),
        length,
        client: u16v(8),
        session: u16v(10),
        protocol_version: b[12],
        interface_version: b[13],
        message_type: b[14],
        return_code: b[15],
        payload: b[16..(8 + length as usize)].to_vec(),
    })
}

#[napi(js_name = "SomeipMessage")]
pub struct SomeipMessage {
    pub service: u16,
    pub instance: u16,
    pub method: u16,
    pub client: u16,
    pub session: u16,
    pub payload: Vec<u8>,
    pub message_type: u8,
    pub return_code: u8,
    pub protocol_version: u8,
    pub interface_version: u8,
    pub reliable: bool,
}
#[napi]
impl SomeipMessage {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            service: 0,
            instance: 0,
            method: 0,
            client: 0,
            session: 0,
            payload: Vec::new(),
            message_type: 0,
            return_code: 0,
            protocol_version: 1,
            interface_version: 1,
            reliable: false,
        }
    }
}

#[napi]
pub struct Runtime {
    apps: Arc<Mutex<HashMap<String, Arc<Mutex<AppState>>>>>,
}
#[napi]
impl Runtime {
    #[napi]
    pub fn get() -> Self {
        Self {
            apps: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    #[napi]
    pub fn create_application(&self, name: String, config: String) -> Application {
        // The native transport accepts either a UDP target (host:port) or a generated config path.
        // Config unicast is read here and the standard SOME/IP port is bound before falling back.
        let target = config.parse::<SocketAddr>().unwrap_or_else(|_| {
            let json = std::fs::read_to_string(&config)
                .ok()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
            let host = json
                .as_ref()
                .and_then(|v| v.get("unicast"))
                .and_then(|v| v.as_str())
                .unwrap_or("127.0.0.1");
            host.parse::<SocketAddr>().unwrap_or_else(|_| {
                format!("{host}:30509")
                    .parse()
                    .unwrap_or_else(|_| "127.0.0.1:30509".parse().unwrap())
            })
        });
        let bind_addr = format!("0.0.0.0:{}", target.port());
        let socket = UdpSocket::bind(&bind_addr)
            .or_else(|_| UdpSocket::bind("0.0.0.0:0"))
            .ok()
            .and_then(|s| {
                let _ = s.set_read_timeout(Some(Duration::from_millis(100)));
                Some(Arc::new(s))
            });
        let sd_host = json_sd_host(&config).unwrap_or_else(|| "224.0.0.1".into());
        let sd_port = json_sd_port(&config).unwrap_or(30490);
        let sd_target = format!("{sd_host}:{sd_port}")
            .parse()
            .unwrap_or_else(|_| "224.0.0.1:30490".parse().unwrap());
        let endpoint = socket.map(|s| Endpoint {
            socket: s,
            target,
            sd_target,
            reliable: false,
        });
        let st = Arc::new(Mutex::new(AppState {
            endpoint,
            offers: HashMap::new(),
            subscriptions: HashMap::new(),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            worker: None,
            periodic: HashMap::new(),
            handler_ids: HashMap::new(),
        }));
        self.apps.lock().unwrap().insert(name.clone(), st.clone());
        apps().lock().unwrap().insert(name.clone(), st);
        Application { name }
    }
    #[napi]
    pub fn remove_application(&self, name: String) {
        if let Some(st) = self.apps.lock().unwrap().remove(&name) {
            let _ = stop_state(&st);
        }
        apps().lock().unwrap().remove(&name);
    }
}
fn json_sd_host(config: &str) -> Option<String> {
    let text = std::fs::read_to_string(config).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("service-discovery")?
        .get("multicast")?
        .as_str()
        .map(String::from)
}
fn json_sd_port(config: &str) -> Option<u16> {
    let text = std::fs::read_to_string(config).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("service-discovery")?
        .get("port")?
        .as_u64()
        .and_then(|x| u16::try_from(x).ok())
}
fn stop_state(st: &Arc<Mutex<AppState>>) -> napi::Result<()> {
    let mut s = st
        .lock()
        .map_err(|_| Error::from_reason("state poisoned"))?;
    s.running.store(false, std::sync::atomic::Ordering::Release);
    if let Some(ep) = s.endpoint.clone() {
        let _ = ep.socket.set_read_timeout(Some(Duration::from_millis(1)));
    }
    if let Some(j) = s.worker.take() {
        let _ = j.join();
    }
    for (_, mut task) in s.periodic.drain() {
        task.stop.store(true, std::sync::atomic::Ordering::Release);
        if let Some(j) = task.worker.take() {
            let _ = j.join();
        }
    }
    Ok(())
}
#[napi]
pub struct Application {
    name: String,
}
#[napi]
impl Application {
    #[napi]
    pub fn init(&self) -> bool {
        if let Ok(st) = state(&self.name) {
            if let Ok(mut s) = st.lock() {
                if let Some(ep) = s.endpoint.as_ref() {
                    let _ = ep.socket.set_broadcast(true);
                }
            }
            emit_event(&self.name, "state", CallbackData { service: None, instance: None, method: None, available: None, subscription_id: None, status: Some(0), text: None, message: None });
            true
        } else {
            false
        }
    }
    #[napi]
    pub fn clear_all_handler(&self) {
        if let Ok(st) = state(&self.name) {
            if let Ok(mut value) = st.lock() {
                value.handler_ids.clear();
            }
        }
    }
    #[napi]
    pub fn offer_service(
        &self,
        service: u16,
        instance: u16,
        major: Option<u8>,
        minor: Option<u32>,
        ttl: Option<u32>,
    ) -> Result<()> {
        let st = state(&self.name)?;
        st.lock()
            .map_err(|_| Error::from_reason("state poisoned"))?
            .offers
            .insert(
                (service, instance),
                ServiceOffer {
                    service,
                    instance,
                    major: major.unwrap_or(0),
                    minor: minor.unwrap_or(0),
                    ttl: ttl.unwrap_or(0xffff_ffff),
                },
            );
        if let Some(ep) = st
            .lock()
            .map_err(|_| Error::from_reason("state poisoned"))?
            .endpoint
            .clone()
        {
            send_sd(
                &ep,
                sd_entry(
                    1,
                    service,
                    instance,
                    major.unwrap_or(0),
                    minor.unwrap_or(0),
                    ttl.unwrap_or(0xffff_ffff),
                    0,
                ),
            )?;
        }
        Ok(())
    }
    #[napi]
    pub fn stop_offer_service(&self, service: u16, instance: u16) -> Result<()> {
        let st = state(&self.name)?;
        st.lock()
            .map_err(|_| Error::from_reason("state poisoned"))?
            .offers
            .remove(&(service, instance));
        if let Some(ep) = st
            .lock()
            .map_err(|_| Error::from_reason("state poisoned"))?
            .endpoint
            .clone()
        {
            send_sd(&ep, sd_entry(2, service, instance, 0, 0, 0, 0))?;
        }
        Ok(())
    }
    #[napi]
    pub fn request_service(
        &self,
        service: u16,
        instance: u16,
        major: Option<u8>,
        minor: Option<u32>,
    ) -> Result<bool> {
        let st = state(&self.name)?;
        if let Some(ep) = st
            .lock()
            .map_err(|_| Error::from_reason("state poisoned"))?
            .endpoint
            .clone()
        {
            send_sd(
                &ep,
                sd_entry(
                    0,
                    service,
                    instance,
                    major.unwrap_or(0),
                    minor.unwrap_or(0),
                    0xffff_ffff,
                    0,
                ),
            )?;
        }
        let available = st
            .lock()
            .map_err(|_| Error::from_reason("state poisoned"))?
            .offers
            .get(&(service, instance))
            .map(|o| o.major == major.unwrap_or(0) && o.minor >= minor.unwrap_or(0))
            .unwrap_or(false);
        emit_event(&self.name, "availability", CallbackData { service: Some(service), instance: Some(instance), method: None, available: Some(available), subscription_id: None, status: None, text: None, message: None });
        Ok(available)
    }
    #[napi]
    pub fn subscribe(
        &self,
        service: u16,
        instance: u16,
        eventgroup: u16,
        _major: Option<u8>,
        event: Option<u16>,
    ) -> Result<String> {
        let st = state(&self.name)?;
        if let Some(ep) = st
            .lock()
            .map_err(|_| Error::from_reason("state poisoned"))?
            .endpoint
            .clone()
        {
            send_sd(
                &ep,
                sd_entry(6, service, instance, 0, 0, 0xffff_ffff, eventgroup),
            )?;
        }
        let id = uuid::Uuid::new_v4().to_string();
        state(&self.name)?
            .lock()
            .map_err(|_| Error::from_reason("state poisoned"))?
            .subscriptions
            .insert(
                id.clone(),
                Subscription {
                    service,
                    instance,
                    eventgroup,
                    ttl: event.unwrap_or(0xffff) as u32,
                },
            );
        emit_event(&self.name, "subscription", CallbackData { service: Some(service), instance: Some(instance), method: Some(eventgroup), available: None, subscription_id: Some(id.clone()), status: Some(0), text: None, message: None });
        Ok(id)
    }
    #[napi]
    pub fn release_service(&self, service: u16, instance: u16) -> Result<()> {
        self.stop_offer_service(service, instance)
    }
    #[napi]
    pub fn stop_offer_event(&self, service: u16, instance: u16, event: u16) -> Result<()> {
        let st = state(&self.name)?;
        if let Some(ep) = st
            .lock()
            .map_err(|_| Error::from_reason("state poisoned"))?
            .endpoint
            .clone()
        {
            send_sd(&ep, sd_entry(2, service, instance, 0, 0, 0, event))?;
        }
        Ok(())
    }
    #[napi]
    pub fn unsubscribe(&self, id: String) -> Result<()> {
        let removed = state(&self.name)?
            .lock()
            .map_err(|_| Error::from_reason("state poisoned"))?
            .subscriptions
            .remove(&id).is_some();
        emit_event(&self.name, "subscription_status", CallbackData { service: None, instance: None, method: None, available: None, subscription_id: Some(id), status: Some(if removed { 0 } else { -1 }), text: None, message: None });
        Ok(())
    }
}

#[napi]
pub struct VsomeipCallbackWrapper {
    name: String,
}
#[napi]
impl VsomeipCallbackWrapper {
    #[napi(constructor)]
    pub fn new(_r: &Runtime, a: &Application) -> Self {
        Self {
            name: a.name.clone(),
        }
    }
    #[napi]
    pub fn start(&self) -> Result<()> {
        let st = state(&self.name)?;
        let mut s = st
            .lock()
            .map_err(|_| Error::from_reason("state poisoned"))?;
        if s.running.swap(true, std::sync::atomic::Ordering::AcqRel) {
            return Ok(());
        }
        let ep = s
            .endpoint
            .clone()
            .ok_or_else(|| Error::from_reason("UDP endpoint unavailable"))?;
        let running = s.running.clone();
        let app_name = self.name.clone();
        let j = thread::spawn(move || {
            let mut b = [0u8; 65535];
            while running.load(std::sync::atomic::Ordering::Acquire) {
                match ep.socket.recv_from(&mut b) {
                    Ok((n, _)) => {
                        if let Some(v) = decode(&b[..n]) {
                            emit_event(&app_name, "trace", CallbackData { service: Some(v.service), instance: None, method: Some(v.method), available: None, subscription_id: None, status: None, text: Some(format!("received SOME/IP {}:{}", v.service, v.method)), message: Some(v.clone()) });
                            if let Ok(map) = callbacks().lock() {
                                for (id, reg) in map.iter() {
                                    let matched = state(&reg.app).ok().map(|st| st.lock().ok().map(|s| s.handler_ids.iter().any(|(key, h)| key == id && matches!(h, HandlerRegistration::Message { service, instance, method } if (*service == 0xffff || *service == v.service) && (*instance == 0xffff || *instance == 0) && (*method == 0xffff || *method == v.method)))).unwrap_or(false)).unwrap_or(false);
                                    if matched {
                                        let _ = reg.callback.call(
                                            CallbackEvent {
                                                kind: "message".into(),
                                                data: CallbackData {
                                                    service: Some(v.service), instance: None, method: Some(v.method),
                                                    available: None, subscription_id: None, status: None, text: None,
                                                    message: Some(v.clone()),
                                                },
                                            },
                                            ThreadsafeFunctionCallMode::NonBlocking,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(_) => break,
                }
            }
        });
        s.worker = Some(j);
        Ok(())
    }
    #[napi]
    pub fn stop(&self) -> Result<()> {
        stop_state(&state(&self.name)?)
    }
    #[napi]
    pub fn is_running(&self) -> bool {
        state(&self.name)
            .map(|s| {
                s.lock()
                    .unwrap()
                    .running
                    .load(std::sync::atomic::Ordering::Acquire)
            })
            .unwrap_or(false)
    }
    #[napi]
    pub fn register_trace_handler(&self, id: String) -> bool {
        self.register_handler(id, HandlerRegistration::Trace)
    }
    #[napi]
    pub fn register_state_handler(&self, id: String) {
        let _ = self.register_handler(id, HandlerRegistration::State);
    }
    #[napi]
    pub fn register_message_handler(&self, service: u16, instance: u16, method: u16, id: String) {
        let _ = self.register_handler(
            id,
            HandlerRegistration::Message {
                service,
                instance,
                method,
            },
        );
    }
    #[napi]
    pub fn register_availability_handler(&self, service: u16, instance: u16, id: String) {
        let _ = self.register_handler(id, HandlerRegistration::Availability { service, instance });
    }
    fn register_handler(&self, id: String, handler: HandlerRegistration) -> bool {
        state(&self.name)
            .and_then(|st| {
                st.lock()
                    .map_err(|_| Error::from_reason("state poisoned"))?
                    .handler_ids
                    .insert(id, handler);
                Ok(())
            })
            .is_ok()
    }
}

#[napi]
pub struct Send {
    name: String,
}
#[napi]
impl Send {
    #[napi(constructor)]
    pub fn new(_r: &Runtime, a: &Application) -> Self {
        Self {
            name: a.name.clone(),
        }
    }
    #[napi]
    pub fn send_message(&self, m: &SomeipMessage, payload: Buffer) -> Result<()> {
        let st = state(&self.name)?;
        let ep = st
            .lock()
            .unwrap()
            .endpoint
            .clone()
            .ok_or_else(|| Error::from_reason("UDP endpoint unavailable"))?;
        let packet = encode(m, &payload);
        if m.reliable || ep.reliable {
            let mut stream = TcpStream::connect(ep.target)
                .map_err(|e| Error::from_reason(format!("vSomeIP TCP connect: {e}")))?;
            use std::io::Write;
            stream
                .write_all(&packet)
                .map_err(|e| Error::from_reason(format!("vSomeIP TCP send: {e}")))?;
        } else {
            ep.socket
                .send_to(&packet, ep.target)
                .map_err(|e| Error::from_reason(format!("vSomeIP UDP send: {e}")))?;
        }
        Ok(())
    }
    #[napi]
    pub fn start_periodic_message(
        &self,
        id: String,
        m: &SomeipMessage,
        payload: Buffer,
        period: u32,
        _notify: bool,
        _force: bool,
    ) -> Result<()> {
        if period == 0 {
            return Err(Error::from_reason("period must be positive"));
        }
        let st = state(&self.name)?;
        let ep = st
            .lock()
            .unwrap()
            .endpoint
            .clone()
            .ok_or_else(|| Error::from_reason("UDP endpoint unavailable"))?;
        let msg = encode(m, &payload);
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = stop.clone();
        let j = thread::spawn(move || {
            while !flag.load(std::sync::atomic::Ordering::Acquire) {
                let _ = ep.socket.send_to(&msg, ep.target);
                thread::sleep(Duration::from_millis(period as u64));
            }
        });
        let mut s = st.lock().unwrap();
        if let Some(mut old) = s.periodic.insert(
            id,
            Periodic {
                stop,
                worker: Some(j),
                period: Duration::from_millis(period as u64),
            },
        ) {
            old.stop.store(true, std::sync::atomic::Ordering::Release);
            if let Some(j) = old.worker.take() {
                let _ = j.join();
            }
        }
        Ok(())
    }
    #[napi]
    pub fn update_periodic_message(
        &self,
        id: String,
        m: &SomeipMessage,
        payload: Buffer,
        notify: bool,
        force: bool,
    ) -> Result<()> {
        let st = state(&self.name)?;
        let period = st
            .lock()
            .unwrap()
            .periodic
            .get(&id)
            .map(|task| task.period)
            .ok_or_else(|| Error::from_reason("periodic task not found"))?;
        let millis = period.as_millis().max(1).min(u32::MAX as u128) as u32;
        self.stop_periodic_message(id.clone())?;
        self.start_periodic_message(id, m, payload, millis, notify, force)
    }
    #[napi]
    pub fn notify_event(
        &self,
        service: u16,
        instance: u16,
        event: u16,
        payload: Buffer,
        _force: bool,
    ) -> Result<()> {
        let mut m = SomeipMessage::new();
        m.service = service;
        m.instance = instance;
        m.method = event;
        m.message_type = 2;
        self.send_message(&m, payload)
    }
    #[napi]
    pub fn request_event_one_group(
        &self,
        service: u16,
        instance: u16,
        event: u16,
        eventgroup: u16,
        _event_type: u8,
    ) -> Result<()> {
        let st = state(&self.name)?;
        if let Some(ep) = st
            .lock()
            .map_err(|_| Error::from_reason("state poisoned"))?
            .endpoint
            .clone()
        {
            send_sd(
                &ep,
                sd_entry(5, service, instance, 0, 0, 0xffff_ffff, eventgroup),
            )?;
        }
        let _ = event;
        Ok(())
    }
    #[napi]
    pub fn release_event_simple(&self, service: u16, instance: u16, event: u16) -> Result<()> {
        self.request_event_one_group(service, instance, event, 0, 0)
    }
    #[napi]
    pub fn offer_event_with_groups(
        &self,
        service: u16,
        instance: u16,
        event: u16,
        groups: String,
        _event_type: u8,
    ) -> Result<()> {
        for group in groups.split(',').filter_map(|x| x.parse().ok()) {
            let st = state(&self.name)?;
            let ep = st
                .lock()
                .map_err(|_| Error::from_reason("state poisoned"))?
                .endpoint
                .clone();
            if let Some(ep) = ep {
                send_sd(
                    &ep,
                    sd_entry(4, service, instance, 0, 0, 0xffff_ffff, group),
                )?;
            }
        }
        let _ = event;
        Ok(())
    }
    #[napi]
    pub fn stop_periodic_message(&self, id: String) -> Result<()> {
        let st = state(&self.name)?;
        let mut s = st.lock().unwrap();
        if let Some(mut task) = s.periodic.remove(&id) {
            task.stop.store(true, std::sync::atomic::Ordering::Release);
            if let Some(j) = task.worker.take() {
                let _ = j.join();
            }
        }
        Ok(())
    }
}

#[napi(js_name = "RegisterCallback")]
pub fn register_callback(name: String, _key: String, cb: Function<'static>) -> Result<String> {
    let tsfn = cb
        .build_threadsafe_function()
        .build_callback(|ctx: ThreadsafeCallContext<CallbackEvent>| {
            Ok((ctx.value.kind, ctx.value.data))
        })?;
    let id = uuid::Uuid::new_v4().to_string();
    callbacks()
        .lock()
        .map_err(|_| Error::from_reason("callback state poisoned"))?
        .insert(
            id.clone(),
            CallbackRegistration {
                app: name,
                callback: tsfn,
            },
        );
    Ok(id)
}
#[napi(js_name = "UnregisterCallback")]
pub fn unregister_callback(id: Option<String>) {
    if let Some(id) = id {
        let _ = callbacks().lock().map(|mut m| m.remove(&id));
    }
}
#[napi]
pub fn migration_status() -> String {
    "Rust SOME/IP UDP transport with correct header, cancellable receive worker, callback IDs and periodic lifecycle".into()
}
