import "./index.css";
import Dashboard from "./components/Dashboard.tsx";
import { HashRouter, Route, Routes } from "react-router-dom";
import Shelf from "./components/Shelf.tsx";
import Library from "./components/Library.tsx";
import Series from "./components/Series.tsx";
import Viewer from "./components/Viewer.tsx";
import ScrollToTop from "./components/ScrollToTop.tsx";
import ServerSettings from "./components/ServerSettings.tsx";
import { ToastProvider, useToast } from "./components/ToastContainer.tsx";
import { webSocketService } from "./services/WebSocketService.ts";
import { useEffect } from "react";

const AppContent: React.FC = () => {
  const { showToast } = useToast();

  useEffect(() => {
    // Connect WebSocket
    webSocketService.connect();

    // Handle progress upload start
    const handleUploadStart = (message: any) => {
      showToast(message.message || "Starting to upload offline progress...", "info", 5000);
    };

    // Handle progress upload end
    const handleUploadEnd = (message: any) => {
      const data = message.data;
      if (data && data.total !== undefined) {
        if (data.failed === 0) {
          showToast(
            `Progress upload complete: ${data.succeeded} entries uploaded successfully`,
            "success",
            5000
          );
        } else {
          showToast(
            `Progress upload complete: ${data.succeeded} succeeded, ${data.failed} failed`,
            data.failed > data.succeeded ? "error" : "warning",
            5000
          );
        }
      } else {
        showToast(message.message || "Progress upload completed", "info", 5000);
      }
    };

    // Handle connection status changes
    const handleConnectionStatus = (message: any) => {
      const data = message.data;
      if (data) {
        if (data.connected) {
          const username = data.username || "user";
          showToast(
            `Connected to Kavita server as ${username}`,
            "success",
            4000
          );
        } else {
          showToast(
            "Disconnected from Kavita server - Offline mode",
            "warning",
            4000
          );
        }
      } else {
        showToast(message.message || "Connection status changed", "info", 4000);
      }
    };

    // Handle caching start
    const handleCachingStart = (message: any) => {
      const data = message.data;
      const seriesId = data?.series_id || "unknown";
      showToast(
        `Starting to cache series ${seriesId}...`,
        "info",
        3000
      );
    };

    // Handle volume cached
    const handleVolumeCached = (message: any) => {
      const data = message.data;
      if (data) {
        const volumeTitle = data.volume_title || "volume";
        const progress = data.progress;
        if (progress) {
          showToast(
            `Cached: ${volumeTitle} (${progress.current}/${progress.total})`,
            "info",
            2000
          );
        } else {
          showToast(
            `Cached: ${volumeTitle}`,
            "info",
            2000
          );
        }
      } else {
        showToast(message.message || "Volume cached", "info", 2000);
      }
    };

    // Handle caching end
    const handleCachingEnd = (message: any) => {
      const data = message.data;
      if (data) {
        const volumesCached = data.volumes_cached || 0;
        const totalVolumes = data.total_volumes || 0;
        const seriesId = data.series_id || "unknown";
        showToast(
          `Finished caching series ${seriesId}: ${volumesCached}/${totalVolumes} volumes`,
          "success",
          4000
        );
      } else {
        showToast(message.message || "Caching completed", "success", 4000);
      }
    };

    // Register event handlers
    webSocketService.on("progress_upload_start", handleUploadStart);
    webSocketService.on("progress_upload_end", handleUploadEnd);
    webSocketService.on("connection_status", handleConnectionStatus);
    webSocketService.on("caching_start", handleCachingStart);
    webSocketService.on("volume_cached", handleVolumeCached);
    webSocketService.on("caching_end", handleCachingEnd);

    // Cleanup on unmount
    return () => {
      webSocketService.off("progress_upload_start", handleUploadStart);
      webSocketService.off("progress_upload_end", handleUploadEnd);
      webSocketService.off("connection_status", handleConnectionStatus);
      webSocketService.off("caching_start", handleCachingStart);
      webSocketService.off("volume_cached", handleVolumeCached);
      webSocketService.off("caching_end", handleCachingEnd);
    };
  }, [showToast]);

  return (
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
  );
};

const App: React.FC = () => {
  return (
    <ToastProvider>
      <AppContent />
    </ToastProvider>
  );
};

export default App;
