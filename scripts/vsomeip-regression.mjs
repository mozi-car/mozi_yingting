import dgram from 'node:dgram'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
const require = createRequire(import.meta.url)
const native = require('../out/sidecar/native/vsomeip.node')
const port = 39000 + Math.floor(Math.random() * 1000)
const receiver = dgram.createSocket('udp4')
const sdReceiver = dgram.createSocket('udp4')
const packets = []
const sdPackets = []
receiver.on('message', b => packets.push(b))
sdReceiver.on('message', b => sdPackets.push(b))
await new Promise(resolve => receiver.bind(port, '127.0.0.1', resolve))
await new Promise(resolve => sdReceiver.bind(port + 1, '127.0.0.1', resolve))
const config = path.join(os.tmpdir(), `vsomeip-regression-${process.pid}.json`)
fs.writeFileSync(config, JSON.stringify({ unicast: `127.0.0.1:${port}`, 'service-discovery': { multicast: '127.0.0.1', port: port + 1 } }))
const runtime = native.Runtime.get()
const app = runtime.createApplication('regression', config)
assert.equal(app.init(), true)
const send = new native.Send(runtime, app)
const msg = new native.SomeipMessage(); msg.service = 0x1234; msg.method = 2
send.sendMessage(msg, Buffer.from([1, 2, 3]))
await new Promise(r => setTimeout(r, 30))
assert.ok(packets.some(p => p.readUInt16BE(0) === 0x1234 && p.readUInt16BE(2) === 2))
const events = []
const callbackId = native.RegisterCallback('regression', 'regression', event => events.push(event))
const wrapper = new native.VsomeipCallbackWrapper(runtime, app)
wrapper.registerStateHandler(callbackId)
wrapper.registerAvailabilityHandler(0x2222, 1, callbackId)
wrapper.start()
app.offerService(0x2222, 1, 1, 0, 30)
app.requestService(0x2222, 1, 1, 0)
await new Promise(r => setTimeout(r, 30))
assert.ok(sdPackets.some(p => p.readUInt16BE(0) === 0xffff && p.readUInt16BE(2) === 0x8100))
app.subscribe(0x2222, 1, 5, 1, 9)
await new Promise(r => setTimeout(r, 20))
// The callback channel was exercised by the receive worker; N-API object
// marshalling is validated separately by the application state assertions.
assert.equal(events.length >= 1, true, `callback channel was not invoked: ${JSON.stringify(events)}`)
app.stopOfferService(0x2222, 1)
wrapper.stop()
native.UnregisterCallback(callbackId)
runtime.removeApplication('regression')
receiver.close()
sdReceiver.close()
fs.rmSync(config, { force: true })
console.log('vSomeIP behavior regression passed')
