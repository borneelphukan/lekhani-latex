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
            <th className="p-4 text-left text-sm font-medium text-gray-500">Platform</th>
            <th className="p-4 text-left text-sm font-medium text-gray-500">Package</th>
            <th className="p-4 text-left text-sm font-medium text-gray-500">Download</th>
          </tr>
        </thead>
        <tbody>
          <tr className="border-b border-border/50 hover:bg-gray-50">
            <td className="p-4 font-medium text-gray-900">Windows</td>
            <td className="p-4 text-gray-500">.exe Installer</td>
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
          <tr className="border-b border-border/50 hover:bg-gray-50">
            <td className="p-4 font-medium text-gray-900">macOS</td>
            <td className="p-4 text-gray-500">.dmg Installer</td>
            <td className="p-4">
              <a 
                href="https://github.com/borneelphukan/lekhani-latex/releases/download/macos-stable/lekhani-latex.dmg"
                className="inline-flex items-center justify-center gap-2 rounded-xl bg-white border-2 border-gray-200 text-gray-900 font-medium hover:border-gray-900 transition-colors py-2"
                target="_blank"
                rel="noopener noreferrer"
              >
                <DownloadIcon size={18} />
                Download macOS
              </a>
            </td>
          </tr>
          <tr className="border-b border-border/50 hover:bg-gray-50">
            <td className="p-4 font-medium text-gray-900">Linux (Debian/Ubuntu)</td>
            <td className="p-4 text-gray-500">.deb Package</td>
            <td className="p-4">
              <a 
                href="https://github.com/borneelphukan/lekhani-latex/releases/download/linux-deb/lekhani-latex_*.deb"
                className="inline-flex items-center justify-center gap-2 rounded-xl bg-white border-2 border-gray-200 text-gray-900 font-medium hover:border-gray-900 transition-colors py-2"
                target="_blank"
                rel="noopener noreferrer"
              >
                <DownloadIcon size={18} />
                Download .deb
              </a>
            </td>
          </tr>
          <tr className="border-b border-border/50 hover:bg-gray-50">
            <td className="p-4 font-medium text-gray-900">Linux (RPM)</td>
            <td className="p-4 text-gray-500">.rpm Package</td>
            <td className="p-4">
              <a 
                href="https://github.com/borneelphukan/lekhani-latex/releases/download-linux-rpm/lekhani-latex-*.rpm"
                className="inline-flex items-center justify-center gap-2 rounded-xl bg-white border-2 border-gray-200 text-gray-900 font-medium hover:border-gray-900 transition-colors py-2"
                target="_blank"
                rel="noopener noreferrer"
              >
                <DownloadIcon size={18} />
                Download .rpm
              </a>
            </td>
          </tr>
          <tr className="border-b border-border/50 hover:bg-gray-50">
            <td className="p-4 font-medium text-gray-900">Linux (AppImage)</td>
            <td className="p-4 text-gray-500">AppImage</td>
            <td className="p-4">
              <a 
                href="https://github.com/borneelphukan/lekhani-latex/releases/download-linux-appimage/lekhani-latex-*-linux-x86_64.AppImage"
                className="inline-flex items-center justify-center gap-2 rounded-xl bg-white border-2 border-gray-200 text-gray-900 font-medium hover:border-gray-900 transition-colors py-2"
                target="_blank"
                rel="noopener noreferrer"
              >
                <DownloadIcon size={18} />
                Download AppImage
              </a>
            </td>
          </tr>
          <tr className="border-b border-border/50 hover:bg-gray-50">
            <td className="p-4 font-medium text-gray-900">Linux (Tarball)</td>
            <td className="p-4 text-gray-500">.tar.gz Archive</td>
            <td className="p-4">
              <a 
                href="https://github.com/borneelphukan/lekhani-latex/releases/download-linux-tarball/lekhani-latex-*.tar.gz"
                className="inline-flex items-center justify-center gap-2 rounded-xl bg-white border-2 border-gray-200 text-gray-900 font-medium hover:border-gray-900 transition-colors py-2"
                target="_blank"
                rel="noopener noreferrer"
              >
                <DownloadIcon size={18} />
                Download .tar.gz
              </a>
            </td>
          </tr>
        </tbody>
      </table>

      <p className="text-center text-sm text-gray-500 mt-4">
        All downloads are free and open source.
      </p>
    </div>
  );
}
