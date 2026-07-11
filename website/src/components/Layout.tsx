import { Outlet, Link } from 'react-router-dom';
import { Download } from 'lucide-react';

export default function Layout() {
  return (
    <div className="min-h-screen flex flex-col bg-[#fcfcfc] selection:bg-brand-200 selection:text-brand-900">
      <header className="sticky top-0 z-50 glass">
        <div className="max-w-5xl mx-auto px-6 h-16 flex items-center justify-between">
          <Link to="/" className="flex items-center gap-3 group">
            <img src="/logo.png" alt="Lekhani Logo" className="w-8 h-8 group-hover:scale-105 transition-transform duration-300" />
            <span className="font-serif font-semibold text-lg tracking-wide text-gray-800">Lekhani</span>
          </Link>
          
          <nav className="flex items-center gap-6">
            <a 
              href="https://github.com/borneelphukan/lekhani-latex" 
              target="_blank" 
              rel="noopener noreferrer"
              className="text-gray-500 hover:text-gray-900 transition-colors flex items-center gap-2 text-sm font-medium"
            >
              <span className="hidden sm:inline font-semibold">GitHub</span>
            </a>
            <Link 
              to="/download" 
              className="bg-gray-900 hover:bg-gray-800 text-white px-4 py-2 rounded-full text-sm font-medium transition-all shadow-sm hover:shadow-md flex items-center gap-2"
            >
              <Download size={16} />
              Download
            </Link>
          </nav>
        </div>
      </header>

      <main className="flex-1 flex flex-col">
        <Outlet />
      </main>

      <footer className="border-t border-gray-200 py-12 mt-20">
        <div className="max-w-5xl mx-auto px-6 flex flex-col md:flex-row justify-between items-center gap-6">
          <div className="flex items-center gap-3">
            <img src="/logo.png" alt="Lekhani Logo" className="w-6 h-6 grayscale opacity-60" />
            <span className="font-serif text-gray-500">Lekhani LaTeX</span>
          </div>
          <div className="text-sm text-gray-400">
            © {new Date().getFullYear()} Lekhani. Open source software.
          </div>
        </div>
      </footer>
    </div>
  );
}
