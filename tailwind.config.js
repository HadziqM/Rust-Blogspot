/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./node_modules/flowbite/**/*.js",
    "./pages/templates/**/*.html"
    ],
  theme: {
    extend: {},
  },
  plugins: [
        require('flowbite/plugin'),
        require('flowbite-typography')
    ],
  darkMode: 'class',
}

