/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  darkMode: 'class',
  theme: {
    extend: {
      screens: {
        xs: '420px',
      },
      colors: {
        aethon: {
          bg: 'var(--md-sys-surface, #111215)',
          surface: 'var(--md-sys-surface-container-low, #18191d)',
          elevated: 'var(--md-sys-surface-container-high, #22232a)',
          card: 'var(--md-sys-surface-container, #1e2026)',
          primary: 'var(--md-sys-primary, #79dc9c)',
          accent: 'var(--at-accent, #d0bcff)',
        },
      },
      fontFamily: {
        sans: ['Inter', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'sans-serif'],
        heading: ['Plus Jakarta Sans', 'Inter', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
    },
  },
  plugins: [],
};


