/** Mirrors the rules of `git check-ref-format --branch` closely enough for instant feedback. */
export function refNameError(name: string, existing: string[] = [], kind = 'branch'): string | null {
  if (!name) return null
  if (/\s/.test(name)) return `${capitalize(kind)} names can’t contain spaces.`
  if (/(^[./-])|(\.\.)|([~^:?*[\\])|(\/$)|(\/\/)|(\.lock$)|(@\{)|(\.$)/.test(name)) return `That isn’t a valid ${kind} name.`
  if (existing.includes(name)) return `A ${kind} with that name already exists.`
  return null
}

function capitalize(s: string) {
  return s[0].toUpperCase() + s.slice(1)
}
