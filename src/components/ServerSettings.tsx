import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { fetchServerSettings, updateServerSettings } from "../services/Api";
import LogViewer from "./LogViewer";

const ServerSettings: React.FC = () => {
  const [serverIP, setServerIP] = useState<string>("");
  const [username, setUsername] = useState<string>("");
  const [password, setPassword] = useState<string>("");
  const [loading, setLoading] = useState<boolean>(true);
  const [saving, setSaving] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [showLogs, setShowLogs] = useState<boolean>(false);
  const navigate = useNavigate();

  useEffect(() => {
    const getServerSettings = async () => {
      try {
        setLoading(true);
        const data = await fetchServerSettings();
        setServerIP(data.ip);
        setUsername(data.username);
        setError(null);
      } catch (err) {
        if (err instanceof Error) {
          setError(err.message);
        } else {
          setError("An unexpected error occurred");
        }
      } finally {
        setLoading(false);
      }
    };

    getServerSettings();
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    try {
      setSaving(true);
      setError(null);
      setSuccess(null);
      
      // Validate IP format
      if (serverIP) {
        if (serverIP.includes(':')) {
          const [host, port] = serverIP.split(':');
          const portNum = parseInt(port);
          if (isNaN(portNum) || portNum < 1 || portNum > 65535) {
            setError("Invalid port number. Must be between 1 and 65535");
            setSaving(false);
            return;
          }
        }
      }
      
      // Only send fields that have been changed
      const settings: { ip?: string; username?: string; password?: string } = {};
      
      // Always send the IP address, even if it hasn't changed
      settings.ip = serverIP;
      
      // Only send username and password if they're not empty
      if (username) settings.username = username;
      if (password) settings.password = password;
      
      console.log("Submitting server settings:", { ...settings, password: password ? "******" : undefined });
      
      // Show connecting message
      setSuccess("Terminating current connection and connecting to new server...");
      
      const result = await updateServerSettings(settings);
      console.log("Server settings update result:", result);
      
      // If the response includes current settings, update the UI
      if (result.current_settings) {
        setServerIP(result.current_settings.ip);
        setUsername(result.current_settings.username);
        console.log("Updated settings from response:", result.current_settings);
      }
      
      setSuccess(result.message || "Settings updated successfully");
      
      // Clear password field after successful update
      setPassword("");
    } catch (err) {
      console.error("Error updating server settings:", err);
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError("An unexpected error occurred");
      }
    } finally {
      setSaving(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Backspace" && e.target === document.body) {
      navigate(-1);
    }
  };

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown as any);
    return () => {
      window.removeEventListener("keydown", handleKeyDown as any);
    };
  }, []);

  return (
    <div className="w-full min-h-screen bg-zinc-900 text-white p-6">
      <h1 className="text-3xl font-bold mb-8 text-center">Settings</h1>
      
      {loading ? (
        <div className="text-center text-xl">Loading settings...</div>
      ) : (
        <form onSubmit={handleSubmit} className="w-full">
          {error && (
            <div className="bg-red-500 text-white p-4 mb-6 rounded">
              Error: {error}
            </div>
          )}
          
          {success && (
            <div className="bg-green-500 text-white p-4 mb-6 rounded">
              {success}
            </div>
          )}
          
          <div className="mb-6">
            <label className="block mb-2 text-xl">Server IP Address:</label>
            <input
              type="text"
              value={serverIP}
              onChange={(e) => setServerIP(e.target.value)}
              className="w-full p-4 bg-zinc-800 border border-zinc-700 rounded text-white text-lg"
              placeholder="e.g. 192.168.1.100:5001"
            />
          </div>
          
          <div className="mb-6">
            <label className="block mb-2 text-xl">Username:</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              className="w-full p-4 bg-zinc-800 border border-zinc-700 rounded text-white text-lg"
            />
          </div>
          
          <div className="mb-8">
            <label className="block mb-2 text-xl">Password:</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="w-full p-4 bg-zinc-800 border border-zinc-700 rounded text-white text-lg"
              placeholder="Leave empty to keep current password"
            />
          </div>
          
          <div className="flex justify-between mb-8">
            <button
              type="button"
              onClick={() => navigate(-1)}
              className="bg-zinc-700 hover:bg-zinc-600 text-white py-4 px-8 rounded text-lg"
            >
              Back
            </button>
            
            <button
              type="submit"
              disabled={saving}
              className={`bg-blue-600 hover:bg-blue-500 text-white py-4 px-8 rounded text-lg ${
                saving ? "opacity-50 cursor-not-allowed" : ""
              }`}
            >
              {saving ? "Saving..." : "Save Settings"}
            </button>
          </div>
          
          <div className="mt-8">
            <button
              type="button"
              onClick={() => setShowLogs(true)}
              className="w-full bg-purple-600 hover:bg-purple-500 text-white py-4 px-8 rounded text-lg"
            >
              Show Backend Logs
            </button>
          </div>
        </form>
      )}
      
      <LogViewer isOpen={showLogs} onClose={() => setShowLogs(false)} />
    </div>
  );
};

export default ServerSettings; 