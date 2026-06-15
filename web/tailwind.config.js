/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{js,ts,svelte}'],
  theme: {
    extend: {
      colors: {
        // Background colors
        'bg-primary': '#0d1117',
        'bg-secondary': '#161b22',
        'bg-tertiary': '#1c2129',
        // Border colors
        'border-default': '#30363d',
        'border-muted': '#21262d',
        // Text colors
        'text-primary': '#e1e4e8',
        'text-secondary': '#c9d1d9',
        'text-muted': '#8b949e',
        'text-faint': '#484f58',
        // Accent colors
        'accent-blue': '#58a6ff',
        'accent-green': '#3fb950',
        'accent-yellow': '#d29922',
        'accent-red': '#f85149',
        'accent-purple': '#a371f7',
        'accent-pink': '#f778ba',
        // Subtle background tints
        'accent-blue-subtle': '#1f6feb22',
        'accent-yellow-subtle': '#d2992222',
        'accent-purple-subtle': '#a371f722',
        'accent-red-subtle': '#f8514922',
        'accent-green-subtle': '#3fb95022',
        // Button colors
        'btn-green': '#238636',
        'btn-green-hover': '#2ea043',
      },
      fontFamily: {
        sans: ['Inter', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'sans-serif'],
      },
    },
  },
  plugins: [],
};
