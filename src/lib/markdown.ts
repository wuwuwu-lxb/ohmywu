function escapeHtml(input: string): string {
  return input
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
}

function renderInline(text: string): string {
  let html = escapeHtml(text)
  html = html.replace(/`([^`]+)`/g, "<code>$1</code>")
  html = html.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
  html = html.replace(/\*([^*]+)\*/g, "<em>$1</em>")
  html = html.replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, '<a href="$2" target="_blank" rel="noreferrer">$1</a>')
  return html
}

export function renderMarkdown(input: string): string {
  const lines = input.replace(/\r\n/g, "\n").split("\n")
  const html: string[] = []
  let inCode = false
  let codeBuffer: string[] = []
  let listType: "ul" | "ol" | null = null
  let quoteBuffer: string[] = []

  const flushCode = () => {
    if (!inCode) return
    html.push(`<pre><code>${escapeHtml(codeBuffer.join("\n"))}</code></pre>`)
    inCode = false
    codeBuffer = []
  }

  const flushList = () => {
    if (!listType) return
    html.push(`</${listType}>`)
    listType = null
  }

  const flushQuote = () => {
    if (!quoteBuffer.length) return
    html.push(`<blockquote>${quoteBuffer.map(renderInline).join("<br/>")}</blockquote>`)
    quoteBuffer = []
  }

  for (const rawLine of lines) {
    const line = rawLine.trimEnd()
    const trimmed = line.trim()

    if (trimmed.startsWith("```")) {
      flushQuote()
      flushList()
      if (inCode) {
        flushCode()
      } else {
        inCode = true
      }
      continue
    }

    if (inCode) {
      codeBuffer.push(line)
      continue
    }

    if (!trimmed) {
      flushQuote()
      flushList()
      continue
    }

    if (trimmed.startsWith("> ")) {
      flushList()
      quoteBuffer.push(trimmed.slice(2))
      continue
    }
    flushQuote()

    const heading = trimmed.match(/^(#{1,3})\s+(.+)$/)
    if (heading) {
      flushList()
      const level = heading[1].length
      html.push(`<h${level}>${renderInline(heading[2])}</h${level}>`)
      continue
    }

    const ordered = trimmed.match(/^(\d+)\.\s+(.+)$/)
    if (ordered) {
      if (listType !== "ol") {
        flushList()
        listType = "ol"
        html.push("<ol>")
      }
      html.push(`<li>${renderInline(ordered[2])}</li>`)
      continue
    }

    const unordered = trimmed.match(/^[-*]\s+(.+)$/)
    if (unordered) {
      if (listType !== "ul") {
        flushList()
        listType = "ul"
        html.push("<ul>")
      }
      html.push(`<li>${renderInline(unordered[1])}</li>`)
      continue
    }

    flushList()
    html.push(`<p>${renderInline(trimmed)}</p>`)
  }

  flushQuote()
  flushList()
  flushCode()

  return html.join("")
}
