export const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value))

const toHex = (value: number) => value.toString(16).padStart(2, "0")

export const rgbToHex = (r: number, g: number, b: number) =>
  `#${toHex(clamp(Math.round(r), 0, 255))}${toHex(clamp(Math.round(g), 0, 255))}${toHex(clamp(Math.round(b), 0, 255))}`

export const hexToRgb = (hex: string): [number, number, number] => {
  const value = hex.trim().replace("#", "")
  const normalized = value.length === 3
    ? value.split("").map((char) => char + char).join("")
    : value
  const numeric = parseInt(normalized, 16)
  return [(numeric >> 16) & 255, (numeric >> 8) & 255, numeric & 255]
}

export const rgbToHsl = (r: number, g: number, b: number) => {
  const nr = r / 255
  const ng = g / 255
  const nb = b / 255
  const max = Math.max(nr, ng, nb)
  const min = Math.min(nr, ng, nb)
  const lightness = (max + min) / 2
  const delta = max - min

  if (delta === 0) {
    return { h: 0, s: 0, l: lightness }
  }

  const saturation =
    lightness > 0.5 ? delta / (2 - max - min) : delta / (max + min)

  let hue = 0
  switch (max) {
    case nr:
      hue = (ng - nb) / delta + (ng < nb ? 6 : 0)
      break
    case ng:
      hue = (nb - nr) / delta + 2
      break
    default:
      hue = (nr - ng) / delta + 4
      break
  }

  return { h: hue / 6, s: saturation, l: lightness }
}

const hueToRgb = (p: number, q: number, t: number) => {
  let next = t
  if (next < 0) next += 1
  if (next > 1) next -= 1
  if (next < 1 / 6) return p + (q - p) * 6 * next
  if (next < 1 / 2) return q
  if (next < 2 / 3) return p + (q - p) * (2 / 3 - next) * 6
  return p
}

export const hslToRgb = (h: number, s: number, l: number) => {
  if (s === 0) {
    const v = Math.round(l * 255)
    return { r: v, g: v, b: v }
  }

  const q = l < 0.5 ? l * (1 + s) : l + s - l * s
  const p = 2 * l - q

  return {
    r: Math.round(hueToRgb(p, q, h + 1 / 3) * 255),
    g: Math.round(hueToRgb(p, q, h) * 255),
    b: Math.round(hueToRgb(p, q, h - 1 / 3) * 255),
  }
}

export const mixRgb = (
  left: [number, number, number],
  right: [number, number, number],
  weight: number,
): [number, number, number] => {
  const ratio = clamp(weight, 0, 1)
  return [
    Math.round(left[0] * (1 - ratio) + right[0] * ratio),
    Math.round(left[1] * (1 - ratio) + right[1] * ratio),
    Math.round(left[2] * (1 - ratio) + right[2] * ratio),
  ]
}

export const mixHex = (left: string, right: string, weight: number) => {
  const mixed = mixRgb(hexToRgb(left), hexToRgb(right), weight)
  return rgbToHex(mixed[0], mixed[1], mixed[2])
}

export const shiftLightness = (hex: string, targetLightness: number, saturationBoost = 0) => {
  const [r, g, b] = hexToRgb(hex)
  const { h, s } = rgbToHsl(r, g, b)
  const next = hslToRgb(h, clamp(s + saturationBoost, 0, 1), clamp(targetLightness, 0, 1))
  return rgbToHex(next.r, next.g, next.b)
}

const loadImage = (url: string) =>
  new Promise<HTMLImageElement>((resolve, reject) => {
    const image = new Image()
    image.crossOrigin = "anonymous"
    image.decoding = "async"
    image.onload = () => resolve(image)
    image.onerror = () => reject(new Error("Image load failed"))
    image.src = url
  })

export async function extractDominantAccentFromImage(url: string): Promise<string | null> {
  if (!url) return null

  const image = await loadImage(url)
  const canvas = document.createElement("canvas")
  const size = 48
  canvas.width = size
  canvas.height = size

  const context = canvas.getContext("2d", { willReadFrequently: true })
  if (!context) return null

  context.drawImage(image, 0, 0, image.naturalWidth, image.naturalHeight, 0, 0, size, size)
  const pixels = context.getImageData(0, 0, size, size).data
  const buckets = new Map<string, { r: number; g: number; b: number; count: number }>()

  for (let index = 0; index < pixels.length; index += 4) {
    const r = pixels[index]
    const g = pixels[index + 1]
    const b = pixels[index + 2]
    const a = pixels[index + 3]

    if (a < 180) continue

    const { s, l } = rgbToHsl(r, g, b)
    if (l < 0.08 || l > 0.92) continue
    if (s < 0.04 && l > 0.78) continue

    const key = `${Math.round(r / 24)}-${Math.round(g / 24)}-${Math.round(b / 24)}`
    const current = buckets.get(key)

    if (current) {
      current.r += r
      current.g += g
      current.b += b
      current.count += 1
      continue
    }

    buckets.set(key, { r, g, b, count: 1 })
  }

  let winner: { r: number; g: number; b: number; count: number } | null = null
  let bestScore = -1

  for (const bucket of buckets.values()) {
    const average = {
      r: bucket.r / bucket.count,
      g: bucket.g / bucket.count,
      b: bucket.b / bucket.count,
    }
    const { s, l } = rgbToHsl(average.r, average.g, average.b)
    const contrastWindow = 1 - Math.min(1, Math.abs(l - 0.56) * 1.8)
    const richness = Math.max(average.r, average.g, average.b) - Math.min(average.r, average.g, average.b)
    const score = bucket.count * (1 + s * 1.8 + contrastWindow * 0.6) + richness * 0.45

    if (score > bestScore) {
      bestScore = score
      winner = bucket
    }
  }

  canvas.remove()
  image.remove()

  if (!winner) return null

  const average = {
    r: winner.r / winner.count,
    g: winner.g / winner.count,
    b: winner.b / winner.count,
  }
  const { h, s, l } = rgbToHsl(average.r, average.g, average.b)
  const accentSaturation = clamp(Math.max(s, 0.34), 0.34, 0.78)
  const accentLightness = clamp(l < 0.42 ? l + 0.18 : l > 0.72 ? l - 0.12 : l, 0.48, 0.68)
  const accent = hslToRgb(h, accentSaturation, accentLightness)

  return rgbToHex(accent.r, accent.g, accent.b)
}
