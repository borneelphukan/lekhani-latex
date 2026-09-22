export default function Footer() {
  return (
    <footer className="border-t border-gray-200 py-12 mt-20">
      <div className="max-w-5xl mx-auto px-6 flex flex-col md:flex-row justify-between items-center gap-6">
        <div className="flex items-center gap-3">
          <img src={`${import.meta.env.BASE_URL}logo.png`} alt="Lekhani Logo" className="w-6 h-6 grayscale opacity-60" />
          <span className="font-serif text-gray-500">Lekhani LaTeX</span>
        </div>
        <div className="text-sm text-gray-400">
          © {new Date().getFullYear()} Lekhani. Open source software.
        </div>
      </div>
    </footer>
  );
}
