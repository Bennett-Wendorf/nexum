import MarkdownIt from 'markdown-it';

const md = new MarkdownIt({
  html: false,
  linkify: true,
  typographer: true,
  breaks: true,
});

/**
 * Renders markdown text to HTML string.
 * @param text - Markdown text to render
 * @returns HTML string
 */
export function renderMarkdown(text: string): string {
  if (!text) return '';
  return md.render(text);
}
