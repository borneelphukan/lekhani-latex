# Lekhani LaTeX

Lekhani is a free desktop app for writing LaTeX documents. Write on one side of the window and preview the finished PDF on the other.

[Download Lekhani](https://github.com/borneelphukan/lekhani-latex/releases) · [Project website](https://borneelphukan.github.io/lekhani-latex/) · [Get help or report a problem](https://github.com/borneelphukan/lekhani-latex/issues)

## What you can do

- Write and edit multiple LaTeX files in tabs.
- See LaTeX commands and document structure highlighted as you type.
- Get suggestions for LaTeX commands while typing.
- Compile your document and view the PDF inside Lekhani.
- Turn on **Auto-compile** to refresh the PDF whenever you save.
- Change the editor theme, and zoom or move between PDF pages.

## Before you begin

Lekhani is the editor; a LaTeX distribution is needed to turn your document into a PDF. Install one for your computer before compiling:

- **Windows:** [MiKTeX](https://miktex.org/)
- **macOS:** [MacTeX](https://www.tug.org/mactex/) or [TeX Live](https://tug.org/texlive/)
- **Linux:** [TeX Live](https://tug.org/texlive/)

The built-in PDF preview also needs a PDF rendering tool. Lekhani can use [MuPDF](https://mupdf.com/), [Ghostscript](https://ghostscript.com/), or [Poppler](https://poppler.freedesktop.org/). If none is installed, compilation can still create a PDF, but it will not appear in Lekhani's preview pane.

## Get started

1. [Download Lekhani](https://github.com/borneelphukan/lekhani-latex/releases) for your operating system and install it.
2. Open Lekhani and choose **File → New Document** to start writing, or **File → Open…** to continue an existing `.tex` file.
3. Save your document with **File → Save**.
4. Select **Compile** to create a PDF. The preview appears in the right-hand pane.

## Everyday use

| To… | Do this |
|---|---|
| Create or open a document | Choose **New Document** or **Open…** from the **File** menu |
| Save your work | Choose **Save** or **Save As…** from the **File** menu |
| Create or refresh the PDF | Select **Compile** |
| Refresh the PDF whenever you save | Turn on **Auto-compile** |
| Show or hide the PDF pane | Select **View → Toggle Preview** or use the preview button |
| Move through the PDF | Use the page arrows in the preview pane |
| Make the PDF larger or smaller | Use the zoom controls, or pinch/hold Ctrl while scrolling |
| Switch between light and dark appearance | Choose **Settings → Theme** |

## If something goes wrong

- **Compile is unavailable:** Save the document first and make sure a LaTeX distribution is installed.
- **The PDF does not appear inside Lekhani:** Install one of the PDF rendering tools listed above, then reopen Lekhani. You can also open the generated PDF with another PDF viewer.
- **The document reports a missing package:** Install that package using your LaTeX distribution's package manager.

## About the project

Lekhani is free and open source under the [MIT License](LICENSE). You can [browse the source code](https://github.com/borneelphukan/lekhani-latex) or [report a problem](https://github.com/borneelphukan/lekhani-latex/issues).
