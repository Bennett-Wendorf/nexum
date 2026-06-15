import { describe, it, expect } from 'vitest';
import MarkdownIt from 'markdown-it';

describe('Markdown Rendering', () => {
  const md = new MarkdownIt({
    html: false,
    breaks: true,
    linkify: true,
    typographer: true,
  });

  it('renders headings', () => {
    const html = md.render('# Hello');
    expect(html).toContain('<h1>');
  });

  it('renders lists', () => {
    const html = md.render('- Item 1\n- Item 2');
    expect(html).toContain('<ul>');
    expect(html).toContain('<li>');
  });

  it('renders code blocks', () => {
    const html = md.render('```\ncode()\n```');
    expect(html).toContain('<pre>');
  });

  it('does not pass through raw HTML', () => {
    const html = md.render('<script>alert("xss")</script>');
    expect(html).not.toContain('<script>');
  });

  it('auto-links URLs', () => {
    const html = md.render('Check https://example.com for more');
    expect(html).toContain('<a href="https://example.com">');
  });

  it('converts single newlines to breaks', () => {
    const html = md.render('Line 1\nLine 2');
    expect(html).toContain('<br>');
  });
});
