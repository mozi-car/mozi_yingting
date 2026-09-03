use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

struct PortState {
    port: Arc<Mutex<Box<dyn SerialPort + Send>>>,
    stop: Arc<Mutex<bool>>,
    worker: Option<JoinHandle<()>>,
    callback: Option<Arc<Mutex<ThreadsafeFunction<Vec<u8>>>>>,
}

#[napi]
pub struct Serial {
    state: Mutex<Option<PortState>>,
}

#[napi]
impl Serial {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(None),
        }
    }

    #[napi]
    pub fn open(
        &self,
        path: String,
        baud_rate: u32,
        data_bits: Option<u32>,
        stop_bits: Option<u32>,
        parity: Option<String>,
        callback: Option<Function<'static>>,
    ) -> Result<()> {
        let mut builder = serialport::new(&path, baud_rate)
            .timeout(Duration::from_millis(50))
            .data_bits(match data_bits.unwrap_or(8) {
                5 => DataBits::Five,
                6 => DataBits::Six,
                7 => DataBits::Seven,
                _ => DataBits::Eight,
            })
            .stop_bits(if stop_bits.unwrap_or(1) == 2 {
                StopBits::Two
            } else {
                StopBits::One
            })
            .flow_control(FlowControl::None)
            .parity(
                match parity
                    .unwrap_or_else(|| "none".into())
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "even" => Parity::Even,
                    "odd" => Parity::Odd,
                    _ => Parity::None,
                },
            );
        let port = builder
            .open()
            .map_err(|e| Error::from_reason(format!("open serial port {path}: {e}")))?;
        let port: Arc<Mutex<Box<dyn SerialPort + Send>>> = Arc::new(Mutex::new(port));
        let stop = Arc::new(Mutex::new(false));
        let callback: Option<Arc<Mutex<ThreadsafeFunction<Vec<u8>>>>> = match callback {
            Some(function) => Some(Arc::new(Mutex::new(
                function
                    .build_threadsafe_function()
                    .callee_handled()
                    .build_callback(|ctx| Ok(ctx.value))?,
            ))),
            None => None,
        };
        let read_port = Arc::clone(&port);
        let read_stop = Arc::clone(&stop);
        let read_callback = callback.clone();
        let worker = thread::spawn(move || {
            let mut data = [0u8; 4096];
            while !read_stop.lock().map(|v| *v).unwrap_or(true) {
                if let Ok(mut p) = read_port.lock() {
                    match p.read(&mut data) {
                        Ok(n) if n > 0 => {
                            if let Some(cb) = &read_callback {
                                if let Ok(cb) = cb.lock() {
                                    let _ = cb.call(
                                        Ok(data[..n].to_vec()),
                                        ThreadsafeFunctionCallMode::NonBlocking,
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
        *self
            .state
            .lock()
            .map_err(|_| Error::from_reason("serial state poisoned"))? = Some(PortState {
            port,
            stop,
            worker: Some(worker),
            callback,
        });
        Ok(())
    }

    #[napi]
    pub fn write(&self, data: Buffer) -> Result<u32> {
        let state = self
            .state
            .lock()
            .map_err(|_| Error::from_reason("serial state poisoned"))?;
        let port = state
            .as_ref()
            .ok_or_else(|| Error::from_reason("serial port is not open"))?;
        let mut p = port
            .port
            .lock()
            .map_err(|_| Error::from_reason("serial port poisoned"))?;
        p.write_all(&data)
            .map_err(|e| Error::from_reason(format!("serial write: {e}")))?;
        Ok(data.len() as u32)
    }

    #[napi]
    pub fn drain(&self) -> Result<()> {
        let state = self
            .state
            .lock()
            .map_err(|_| Error::from_reason("serial state poisoned"))?;
        let port = state
            .as_ref()
            .ok_or_else(|| Error::from_reason("serial port is not open"))?;
        let result = port
            .port
            .lock()
            .map_err(|_| Error::from_reason("serial port poisoned"))?
            .flush()
            .map_err(|e| Error::from_reason(format!("serial drain: {e}")));
        result
    }

    #[napi]
    pub fn flush(&self) -> Result<()> {
        let state = self
            .state
            .lock()
            .map_err(|_| Error::from_reason("serial state poisoned"))?;
        let port = state
            .as_ref()
            .ok_or_else(|| Error::from_reason("serial port is not open"))?;
        let result = port
            .port
            .lock()
            .map_err(|_| Error::from_reason("serial port poisoned"))?
            .clear(serialport::ClearBuffer::All)
            .map_err(|e| Error::from_reason(format!("serial flush: {e}")));
        result
    }

    #[napi]
    pub fn close(&self) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| Error::from_reason("serial state poisoned"))?;
        if let Some(mut port) = state.take() {
            if let Ok(mut stop) = port.stop.lock() {
                *stop = true;
            }
            if let Some(worker) = port.worker.take() {
                let _ = worker.join();
            }
        }
        Ok(())
    }

    #[napi]
    pub fn is_open(&self) -> bool {
        self.state.lock().map(|s| s.is_some()).unwrap_or(false)
    }
}

#[napi]
pub fn list() -> Result<Vec<String>> {
    Ok(serialport::available_ports()
        .map_err(|e| Error::from_reason(format!("list serial ports: {e}")))?
        .into_iter()
        .map(|p| p.port_name)
        .collect())
}
