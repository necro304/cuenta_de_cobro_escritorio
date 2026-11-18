import { afterEach, expect, it, vi } from 'vitest'
import { getTodayDate } from './format'

afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllEnvs()
})

it('propone el último día del mes a las 19:30 en Bogotá', () => {
  vi.stubEnv('TZ', 'America/Bogota')
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date('2026-11-30T19:30:00-05:00'))
  expect(getTodayDate()).toBe('2026-11-30')
})
