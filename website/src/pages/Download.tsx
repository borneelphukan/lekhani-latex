import { Download as DownloadIcon, Box, Sparkles } from 'lucide-react';

export default function Download() {
  return (
    <div className="flex-1 w-full max-w-5xl mx-auto px-6 py-24 flex flex-col items-center">
      <div className="text-center mb-16">
        <h1 className="text-4xl md:text-5xl font-serif text-gray-900 mb-6">Download Lekhani</h1>
        <p className="text-lg text-gray-500 max-w-xl mx-auto">
          Choose the release channel that best fits your needs. All downloads are free and open source.
        </p>
      </div>

      <div className="grid md:grid-cols-2 gap-8 w-full max-w-4xl">
        {/* Stable Release */}
        <div className="flex flex-col p-8 rounded-3xl bg-white border border-gray-200 shadow-sm hover:shadow-lg transition-all relative overflow-hidden group">
          <div className="absolute top-0 left-0 w-full h-1 bg-brand-500"></div>
          <div className="flex items-center gap-4 mb-6">
            <div className="p-3 bg-brand-50 text-brand-600 rounded-xl">
              <Box size={24} />
            </div>
            <div>
              <h2 className="text-2xl font-serif text-gray-900">Stable Release</h2>
              <span className="text-sm font-medium text-brand-600">From main branch</span>
            </div>
          </div>
          
          <p className="text-gray-500 mb-8 flex-1">
            The recommended choice for most users. Stable releases are thoroughly tested and provide the most reliable experience for your daily writing.
          </p>
          
          <a 
            href="#" 
            className="flex items-center justify-center gap-2 w-full py-4 rounded-xl bg-gray-900 text-white font-medium hover:bg-gray-800 transition-colors"
            onClick={(e) => { e.preventDefault(); alert("Stable release download link would be here."); }}
          >
            <DownloadIcon size={18} />
            Download Stable
          </a>
        </div>

        {/* Nightly Release */}
        <div className="flex flex-col p-8 rounded-3xl bg-white border border-gray-200 shadow-sm hover:shadow-lg transition-all relative overflow-hidden group">
          <div className="absolute top-0 left-0 w-full h-1 bg-purple-500"></div>
          <div className="flex items-center gap-4 mb-6">
            <div className="p-3 bg-purple-50 text-purple-600 rounded-xl">
              <Sparkles size={24} />
            </div>
            <div>
              <h2 className="text-2xl font-serif text-gray-900">Nightly Release</h2>
              <span className="text-sm font-medium text-purple-600">From dev branch</span>
            </div>
          </div>
          
          <p className="text-gray-500 mb-8 flex-1">
            For those who want to test the latest features and improvements. Updated regularly with the newest code from our development branch.
          </p>
          
          <a 
            href="#" 
            className="flex items-center justify-center gap-2 w-full py-4 rounded-xl bg-white border-2 border-gray-200 text-gray-900 font-medium hover:border-gray-900 transition-colors"
            onClick={(e) => { e.preventDefault(); alert("Nightly release download link would be here."); }}
          >
            <DownloadIcon size={18} />
            Download Nightly
          </a>
        </div>
      </div>
    </div>
  );
}
