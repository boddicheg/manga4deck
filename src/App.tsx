import "./index.css";
import Dashboard from "./components/Dashboard.tsx";
import { HashRouter, Route, Routes } from "react-router-dom";
import Shelf from "./components/Shelf.tsx";
import Library from "./components/Library.tsx";
import Series from "./components/Series.tsx";
import Viewer from "./components/Viewer.tsx";

const App: React.FC = () => {
  return (
    <>
      <HashRouter>
        <div className="w-max h-max max-w-full max-h-full">
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/shelf" element={<Shelf />} />
            <Route path="/library/:id" element={<Library />} />
            <Route path="/series/:id" element={<Series />} />
            <Route path="/viewer/:series_id/:volume_id/:chapter_id/:pages/:read" element={<Viewer />} />
          </Routes>
        </div>
      </HashRouter>
    </>
  );
};

export default App;
