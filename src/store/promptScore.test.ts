import { expect, test } from "vitest"

import { score, shouldSkip } from "./promptScore"

test('trivia is skipped like the server does', () => {
  expect(shouldSkip('evet')).toBe(true)
  expect(shouldSkip('/clear and more text')).toBe(true)
  expect(shouldSkip('Evet devam edelim.')).toBe(true)
  expect(shouldSkip('LinePulse raporunu hızlandır lütfen')).toBe(false)
})

test('a vague prompt scores poorly with a context hint', () => {
  const s = score('şuna bir bak düzelt şunu')
  expect(s.label).toBe('Poor')
  expect(s.hint).toContain('dosya')
})

test('a detailed prompt scores well', () => {
  const s = score(
    'src/Reports/DailyReportService.cs içindeki GetTimeline() metodunda SQL timeout hatası alıyorum. ' +
      'Amaç sorguyu covering index ile hızlandırmak. Sadece bu metoda dokun. ' +
      'Beklenen: 2 saniyenin altında dönmeli ve mevcut testler geçmeli.',
  )
  expect(s.context).toBe(25)
  expect(s.verifiability).toBe(18)
  expect(s.label).toBe('Good')
})

test('Turkish dotted I lowercases correctly', () => {
  expect(score('İSTİYORUM ki bu çalışsın, nasıl yaparız').clarity).toBeGreaterThan(8)
})
