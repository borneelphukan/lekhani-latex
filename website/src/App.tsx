import { BrowserRouter, Routes, Route } from 'react-router-dom';
import Layout from './components/Layout';
import Home from './pages/Home';
import Download from './pages/Download';

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<Home />} />
          <Route path="download" element={<Download />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}

export default App;
