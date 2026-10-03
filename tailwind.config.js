/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        aethon: {
          bg: 'var(--at-bg, #111215)',
          surface: 'var(--at-surface, #18191d)',
          elevated: 'var(--at-surface-elevated, #22232a)',
          card: 'var(--at-card, #1e2026)',
          primary: 'var(--at-primary, #79dc9c)',
          accent: 'var(--at-accent, #d0bcff)'
        }
      },
      fontFamily: {
        sans: ['Inter', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'sans-serif'],
        heading: ['Plus Jakarta Sans', 'Inter', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace']
      }
    },
  },
  plugins: [],
}
