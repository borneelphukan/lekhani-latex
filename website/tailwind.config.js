/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'sans-serif'],
        serif: ['"Playfair Display"', 'serif'],
      },
      colors: {
        brand: {
          50: '#f5f7fa',
          100: '#e4ebf3',
          200: '#cbd9e9',
          300: '#a3bfdb',
          400: '#759dcb',
          500: '#5382bc',
          600: '#40669e',
          700: '#345281',
          800: '#2d466b',
          900: '#283b59',
        }
      }
    },
  },
  plugins: [],
}
