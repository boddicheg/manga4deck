import "./index.css";
import Dashboard from "./components/Dashboard.tsx";
import { HashRouter, Route, Routes } from "react-router-dom";
import Shelf from "./components/Shelf.tsx";
import Library from "./components/Library.tsx";
import Series from "./components/Series.tsx";
import Viewer from "./components/Viewer.tsx";
import ScrollToTop from "./components/ScrollToTop.tsx";
import ServerSettings from "./components/ServerSettings.tsx";
import { ToastProvider } from "./components/ToastContainer.tsx";

const App: React.FC = () => {
  return (
    <ToastProvider>
      <HashRouter>
        <div className="w-full h-full">
          <ScrollToTop />
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/shelf" element={<Shelf />} />
            <Route path="/library/:id" element={<Library />} />
            <Route path="/series/:id" element={<Series />} />
            <Route path="/viewer/:series_id/:volume_id/:chapter_id/:pages/:read" element={<Viewer />} />
            <Route path="/settings" element={<ServerSettings />} />
          </Routes>
        </div>
      </HashRouter>
    </ToastProvider>
  );
};

export default App;
