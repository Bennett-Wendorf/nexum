import MarkdownIt from 'markdown-it';

const md = new MarkdownIt({
  html: true,
  linkify: true,
  typographer: true,
  breaks: true,
});

/**
 * Renders markdown text to HTML string.
 * @param {string} text - Markdown text to render
 * @returns {string} HTML string
 */
export function renderMarkdown(text) {
  if (!text) return '';
  return md.render(text);
}
