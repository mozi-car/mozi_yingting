import { getSignal, linStartScheduler } from 'ECB'

Util.Init(async () => {
  await linStartScheduler('NormalTable', 0)
  console.log('start')
})

Util.OnLin('LINdb.MotorControl', () => {
  const val = getSignal('LINdb.MotorDirection')
  console.log('MotorDirection', val)
})

Util.OnSignal('LINdb.MotorSpeed', (val) => {
  console.log('MotorSpeed', val)
})
