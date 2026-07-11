import { Link } from 'react-router-dom';
import { Terminal, FileText, Zap, Eye } from 'lucide-react';

export default function Home() {
  return (
    <div className="flex-1 flex flex-col items-center">
      {/* Hero Section */}
      <section className="w-full max-w-5xl mx-auto px-6 pt-24 pb-20 text-center flex flex-col items-center">
        <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-brand-50 border border-brand-100 text-brand-700 text-xs font-semibold uppercase tracking-wider mb-8">
          <Zap size={14} />
          Cross-platform desktop editor
        </div>
        <h1 className="text-5xl sm:text-6xl md:text-7xl font-serif text-gray-900 leading-tight tracking-tight mb-8 max-w-3xl">
          Writing, through <span className="italic text-brand-700">simplicity.</span>
        </h1>
        <p className="text-lg sm:text-xl text-gray-500 max-w-2xl mb-12 leading-relaxed">
          Lekhani is a native, highly responsive LaTeX editor designed to get out of your way. With live PDF previews, intelligent autocompletion, and robust syntax highlighting.
        </p>
        <div className="flex flex-col sm:flex-row items-center gap-4">
          <Link 
            to="/download" 
            className="bg-gray-900 hover:bg-gray-800 text-white px-8 py-3.5 rounded-full font-medium transition-all shadow-lg hover:shadow-xl hover:-translate-y-0.5"
          >
            Download for Free
          </Link>
          <a 
            href="https://github.com/borneelphukan/lekhani-latex" 
            target="_blank" 
            rel="noopener noreferrer"
            className="bg-white hover:bg-gray-50 text-gray-900 border border-gray-200 px-8 py-3.5 rounded-full font-medium transition-all shadow-sm hover:shadow"
          >
            View Source on GitHub
          </a>
        </div>
      </section>

      {/* App Screenshot Mockup */}
      <section className="w-full max-w-6xl mx-auto px-6 mb-32">
        <div className="relative rounded-2xl md:rounded-[2rem] overflow-hidden shadow-2xl border border-gray-200/50 bg-white aspect-[16/10] flex items-center justify-center">
          {/* Mac window controls mock */}
          <div className="absolute top-0 left-0 w-full h-12 bg-gray-50 border-b border-gray-200 flex items-center px-4 gap-2">
            <div className="w-3 h-3 rounded-full bg-red-400"></div>
            <div className="w-3 h-3 rounded-full bg-yellow-400"></div>
            <div className="w-3 h-3 rounded-full bg-green-400"></div>
            <div className="mx-auto text-sm text-gray-500 font-medium font-sans">main.tex — Lekhani</div>
          </div>
          
          {/* Dummy Content for now */}
          <div className="flex w-full h-full pt-12">
             <div className="w-1/2 h-full border-r border-gray-100 bg-[#1e1e1e] p-8 text-sm font-mono text-gray-300">
               <div className="flex gap-4"><span className="text-gray-500">1</span><span className="text-purple-400">\documentclass</span><span className="text-gray-400">&#123;</span><span>article</span><span className="text-gray-400">&#125;</span></div>
               <div className="flex gap-4"><span className="text-gray-500">2</span></div>
               <div className="flex gap-4"><span className="text-gray-500">3</span><span className="text-purple-400">\begin</span><span className="text-gray-400">&#123;</span><span>document</span><span className="text-gray-400">&#125;</span></div>
               <div className="flex gap-4"><span className="text-gray-500">4</span></div>
               <div className="flex gap-4"><span className="text-gray-500">5</span><span className="text-purple-400">\title</span><span className="text-gray-400">&#123;</span><span>Writing, through simplicity.</span><span className="text-gray-400">&#125;</span></div>
               <div className="flex gap-4"><span className="text-gray-500">6</span><span className="text-purple-400">\author</span><span className="text-gray-400">&#123;</span><span>Lekhani LaTeX</span><span className="text-gray-400">&#125;</span></div>
               <div className="flex gap-4"><span className="text-gray-500">7</span><span className="text-purple-400">\maketitle</span></div>
               <div className="flex gap-4"><span className="text-gray-500">8</span></div>
               <div className="flex gap-4"><span className="text-gray-500">9</span><span>Lekhani is a native, highly responsive LaTeX editor designed to get out of your way.</span></div>
               <div className="flex gap-4"><span className="text-gray-500">10</span></div>
               <div className="flex gap-4"><span className="text-gray-500">11</span><span className="text-purple-400">\end</span><span className="text-gray-400">&#123;</span><span>document</span><span className="text-gray-400">&#125;</span></div>
             </div>
             <div className="w-1/2 h-full bg-gray-50 flex flex-col items-center p-12">
               <div className="bg-white shadow-md w-full max-w-sm h-full rounded p-12 flex flex-col items-center">
                  <h1 className="text-2xl font-serif mb-2">Writing, through simplicity.</h1>
                  <p className="text-sm mb-8">Lekhani LaTeX</p>
                  <p className="text-sm text-justify">Lekhani is a native, highly responsive LaTeX editor designed to get out of your way.</p>
               </div>
             </div>
          </div>
        </div>
      </section>

      {/* Features Grid */}
      <section className="w-full bg-white py-24 border-t border-gray-100">
        <div className="max-w-5xl mx-auto px-6">
          <div className="text-center mb-16">
            <h2 className="text-4xl font-serif mb-4">Everything you need.</h2>
            <p className="text-gray-500 text-lg">Built with egui for maximum performance and minimal footprint.</p>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-8">
            <FeatureCard 
              icon={<Eye size={24} />}
              title="Live PDF Preview"
              description="Compile and view the resulting PDF inline, automatically syncing with your code as you type."
            />
            <FeatureCard 
              icon={<Terminal size={24} />}
              title="Syntax Highlighting"
              description="Commands, math delimiters, braces, and comments are beautifully color-coded."
            />
            <FeatureCard 
              icon={<FileText size={24} />}
              title="Intelligent Autocompletion"
              description="Type `\` followed by a partial command name to instantly see matching LaTeX commands."
            />
          </div>
        </div>
      </section>
    </div>
  );
}

function FeatureCard({ icon, title, description }: { icon: React.ReactNode, title: string, description: string }) {
  return (
    <div className="p-8 rounded-2xl bg-gray-50 border border-gray-100 hover:border-gray-200 hover:bg-gray-100/50 transition-colors group">
      <div className="w-12 h-12 rounded-xl bg-white shadow-sm border border-gray-100 flex items-center justify-center text-brand-600 mb-6 group-hover:scale-110 transition-transform">
        {icon}
      </div>
      <h3 className="text-xl font-serif mb-3 text-gray-900">{title}</h3>
      <p className="text-gray-500 leading-relaxed">{description}</p>
    </div>
  )
}
