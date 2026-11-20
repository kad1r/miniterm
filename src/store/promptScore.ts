// Copied from the prompt-score Claude Code mod (~/.claude/local-mods/prompt-score/
// hooks/score.ts) so the message log shows the same score the mod toasts.
// Keep the two in sync; the toast formatting stays in the mod.

const MAX_DIMENSION = 25
const MIN_LENGTH = 15

const ACKNOWLEDGEMENTS = new Set(['devam edebilirsin', 'evet devam edelim', 'tamamdir devam edelim', 'tamamdır devam edelim'])

const ACTION_VERBS = ['ekle', 'olustur', 'oluştur', 'yaz', 'kaldir', 'kaldır', 'tasi', 'taşı', 'refactor', 'yenile', 'implement', 'fix', 'guncelle', 'güncelle', 'sil', 'degistir', 'değiştir', 'cevir', 'çevir', 'duzenle', 'düzenle']
const VAGUE_VERBS = ['duzelt', 'düzelt', 'iyilestir', 'iyileştir', 'bak', 'hallet', 'guzellestir', 'güzelleştir', 'toparla']
const GOAL_MARKERS = ['amac', 'amaç', 'istiyorum', 'gerekiyor', 'hedef', '?']
const ERROR_MARKERS = ['```', 'exception', 'error', 'hata', 'stack trace', 'traceback']
const TECH_KEYWORDS = ['dotnet', '.net', 'sql', 'ef core', 'react', 'vue', 'azure', 'iis', 'typescript', 'c#', 'http', 'json', 'docker', 'api']
const SCOPE_LIMITERS = ['sadece', 'yalnizca', 'yalnızca', 'dokunma', 'dokun', 'haric', 'hariç', 'degistirme', 'değiştirme']
const ACCEPTANCE_MARKERS = ['beklenen', 'olmali', 'olmalı', 'kabul kriteri', 'expected', 'should']
const OUTPUT_MARKERS = ['ornek cikti', 'örnek çıktı', 'cikti:', 'çıktı:', 'sonuc:', 'sonuç:', 'soyle gorunmeli', 'şöyle görünmeli']

// .NET's \w and \b are Unicode-aware; spell that out for JS.
const W = '[\\p{L}\\p{N}_]'
const FILE_PATH = new RegExp(`[\\p{L}\\p{N}_./\\\\-]+\\.(cs|csproj|ts|js|vue|sql|json|py|md|ps1|yml|yaml|html|css)(?!${W})`, 'iu')
const PATH_SEPARATOR = new RegExp(`[\\p{L}\\p{N}_-]{2,}[/\\\\][\\p{L}\\p{N}_-]{2,}`, 'u')
const IDENTIFIER = new RegExp(`(?<!${W})[a-zA-Z]+[A-Z][A-Za-z0-9]*(?!${W})|(?<!${W})${W}+_${W}+(?!${W})|${W}+\\(\\)`, 'u')

export type Label = 'Poor' | 'Fair' | 'Good'

export type Score = {
  clarity: number
  context: number
  specificity: number
  verifiability: number
  overall: number
  label: Label
  hint: string
}

const clamp = (n: number) => Math.max(0, Math.min(MAX_DIMENSION, n))
const containsAny = (lower: string, needles: readonly string[]) => needles.some(n => lower.includes(n))
const sentences = (text: string) => text.split(/[.!?\n]/).filter(s => s.trim() !== '').length

export const shouldSkip = (text: string): boolean => {
  const trimmed = text.trim()
  if (trimmed.length < MIN_LENGTH || trimmed.startsWith('/')) {
    return true
  }
  const normalized = trimmed.replace(/[.!? ]+$/, '').split(' ').filter(Boolean).join(' ')
  return ACKNOWLEDGEMENTS.has(normalized.toLocaleLowerCase('tr-TR'))
}

const clarityOf = (text: string, lower: string) => {
  let score = 8
  if (containsAny(lower, ACTION_VERBS)) score += 7
  if (sentences(text) >= 2) score += 5
  if (containsAny(lower, GOAL_MARKERS)) score += 5
  if (containsAny(lower, VAGUE_VERBS) && text.length < 60) score -= 8
  return clamp(score)
}

const contextOf = (text: string, lower: string) => {
  let score = 0
  if (FILE_PATH.test(text)) score += 9
  if (containsAny(lower, ERROR_MARKERS)) score += 7
  if (containsAny(lower, TECH_KEYWORDS)) score += 5
  if (PATH_SEPARATOR.test(text)) score += 4
  return clamp(score)
}

const specificityOf = (text: string, lower: string) => {
  let score = text.length < 40 ? 0 : text.length < 120 ? 6 : text.length < 400 ? 12 : 16
  if (IDENTIFIER.test(text)) score += 5
  if (containsAny(lower, SCOPE_LIMITERS)) score += 4
  return clamp(score)
}

const verifiabilityOf = (lower: string) => {
  let score = 0
  if (containsAny(lower, ACCEPTANCE_MARKERS)) score += 10
  if (lower.includes('test')) score += 8
  if (containsAny(lower, OUTPUT_MARKERS)) score += 7
  return clamp(score)
}

// Targets the weakest dimension; ties go Context, Verifiability, Specificity, Clarity.
const hintFor = (clarity: number, context: number, specificity: number, verifiability: number) => {
  const weakest = Math.min(clarity, context, specificity, verifiability)
  if (context === weakest) return 'Hangi dosya, hata mesajı veya kod parçasından bahsettiğini ekle.'
  if (verifiability === weakest) return 'Beklenen sonucu veya kabul kriterini yaz ki sonuç doğrulanabilsin.'
  if (specificity === weakest) return 'Kapsamı daralt: hangi metot veya alan değişecek, neye dokunulmayacak?'
  return 'Ne yapılmasını istediğini tek bir net cümleyle belirt.'
}

export const score = (promptText: string): Score => {
  const text = promptText.trim()
  const lower = text.toLocaleLowerCase('tr-TR')
  const clarity = clarityOf(text, lower)
  const context = contextOf(text, lower)
  const specificity = specificityOf(text, lower)
  const verifiability = verifiabilityOf(lower)
  const overall = clarity + context + specificity + verifiability
  const label: Label = overall < 40 ? 'Poor' : overall < 70 ? 'Fair' : 'Good'
  return { clarity, context, specificity, verifiability, overall, label, hint: hintFor(clarity, context, specificity, verifiability) }
}
