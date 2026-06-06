/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        brand: {
          blue: '#1e90ff',
          payne: '#536878',
          pink: '#f06292',
          hot: '#e91e63',
          violet: '#4a148c',
        },
        accent: {
          DEFAULT: '#1e90ff',
          hover: '#1676d2',
          soft: '#e8f4ff',
          subtle: '#f4faff',
        },
        secondary: {
          DEFAULT: '#f06292',
          hover: '#e24f82',
          soft: '#fff0f6',
        },
        info: {
          DEFAULT: '#4a148c',
          soft: '#f2e9fb',
        },
        success: '#16803c',
        warning: '#a26100',
        danger: '#d1242f',
        bg: {
          primary: '#fbfdff',
          secondary: '#f5f8fb',
          tertiary: '#e8edf3',
          hover: '#d7e1eb',
          card: '#ffffff',
          inset: '#f8fbff',
        },
        border: { DEFAULT: '#d8e0e7', strong: '#b7c4d1' },
        text: {
          primary: '#17202a',
          secondary: '#536878',
          muted: '#8291a3',
        },
      },
      boxShadow: {
        soft: '0 8px 24px rgba(23, 32, 42, 0.08)',
        focus: '0 0 0 3px rgba(30, 144, 255, 0.18)',
      },
    },
  },
  plugins: [require('@tailwindcss/typography')],
}
