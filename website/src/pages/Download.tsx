import { Download as DownloadIcon, Box } from 'lucide-react';

export default function Download() {
  return (
    <div className="flex-1 w-full max-w-5xl mx-auto px-6 py-24 flex flex-col items-center">
      <div className="text-center mb-16">
        <h1 className="text-4xl md:text-5xl font-serif text-gray-900 mb-6">Download Lekhani</h1>
        <p className="text-lg text-gray-500 max-w-xl mx-auto">
          Choose the release channel that best fits your needs. All downloads are free and open source.
        </p>
      </div>

      <table className="w-full max-w-4xl mx-auto border-collapse border-border mb-8">
        <thead>
          <tr className="border-b border-border/50">
            <th className="p-4 text-left text-sm font-medium text-gray-500">Release</th>
            <th className="p-4 text-left text-sm font-medium text-gray-500">Channel</th>
            <th className="p-4 text-left text-sm font-medium text-gray-500">Download</th>
          </tr>
        </thead>
        <tbody>
          <tr className="border-b border-border/50 hover:bg-gray-50">
            <td className="p-4">
              <div className="flex items-center gap-3">
                <div className="p-2 rounded bg-gray-100">
                  <DownloadIcon size={18} />
                </div>
                <div>
                  <h3 className="font-medium text-gray-900">Stable Release</h3>
                  <span className="text-xs text-gray-500">From main branch</span>
                </div>
              </div>
            </td>
            <td className="p-4 text-gray-500">Recommended</td>
            <td className="p-4">
              <a 
                href="https://github.com/borneelphukan/lekhani-latex/releases/download/windows-stable/lekhani-latexSetup.exe"
                className="inline-flex items-center justify-center gap-2 rounded-xl bg-gray-900 text-white font-medium hover:bg-gray-800 transition-colors py-2"
                target="_blank"
                rel="noopener noreferrer"
              >
                <DownloadIcon size={18} />
                Download Stable
              </a>
            </td>
          </tr>
        </tbody>
      </table>

      <p className="text-center text-sm text-gray-500">
        All downloads are free and open source.
      </p>
    </div>
  );
}
