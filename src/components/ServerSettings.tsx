import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { fetchServerSettings, updateServerSettings } from "../services/Api";
import LogViewer from "./LogViewer";
import { useToast } from "./ToastContainer";

const ServerSettings: React.FC = () => {
  const [serverIP, setServerIP] = useState<string>("");
  const [username, setUsername] = useState<string>("");
  const [password, setPassword] = useState<string>("");
  const [hasStoredPassword, setHasStoredPassword] = useState<boolean>(false);
  const [apiKey, setApiKey] = useState<string>("");
  const [loading, setLoading] = useState<boolean>(true);
  const [saving, setSaving] = useState<boolean>(false);
  const [showLogs, setShowLogs] = useState<boolean>(false);
  const navigate = useNavigate();
  const { showToast } = useToast();

  useEffect(() => {
    const getServerSettings = async () => {
      try {
        setLoading(true);
        const data = await fetchServerSettings();
        setServerIP(data.ip);
        setUsername(data.username);
        
        // Set the hasStoredPassword flag if the server has a password stored
        if (data.has_password) {
          setHasStoredPassword(data.has_password);
        }
        
        // Set the API key if it exists
        if (data.api_key) {
          setApiKey(data.api_key);
        }
        
      } catch (err) {
        if (err instanceof Error) {
          showToast(err.message, 'error');
        } else {
          showToast("An unexpected error occurred", 'error');
        }
      } finally {
        setLoading(false);
      }
    };

    getServerSettings();
  }, [showToast]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    try {
      setSaving(true);
      
      // Validate IP format
      if (serverIP) {
        if (serverIP.includes(':')) {
          const [_, port] = serverIP.split(':');
          const portNum = parseInt(port);
          if (isNaN(portNum) || portNum < 1 || portNum > 65535) {
            showToast("Invalid port number. Must be between 1 and 65535", 'error');
            setSaving(false);
            return;
          }
        }
      }
      
      // Only send fields that have been changed
      const settings: { ip?: string; username?: string; password?: string; api_key?: string } = {};
      
      // Always send the IP address, even if it hasn't changed
      settings.ip = serverIP;
      
      // Only send username if it's not empty
      if (username) settings.username = username;
      
      // Only send password if it's not empty (user entered a new one)
      if (password) settings.password = password;
      
      // Always send API key if it exists
      if (apiKey) settings.api_key = apiKey;
      
      console.log("Submitting Kavita settings:", { 
        ...settings, 
        password: password ? "******" : undefined 
      });
      
      // Show connecting message
      showToast("Connecting to Kavita server...", 'info');
      
      const result = await updateServerSettings(settings);
      console.log("Kavita settings update result:", result);
      
      // If the response includes current settings, update the UI
      if (result.current_settings) {
        setServerIP(result.current_settings.ip);
        setUsername(result.current_settings.username);
        
        // Update the hasStoredPassword flag
        if (result.current_settings.has_password !== undefined) {
          setHasStoredPassword(result.current_settings.has_password);
        }
        
        // Update the API key
        if (result.current_settings.api_key) {
          setApiKey(result.current_settings.api_key);
        }
        
        console.log("Updated settings from response:", result.current_settings);
      }
      
      showToast(result.message || "Kavita settings updated successfully", 'success');
      
      // Clear password field after successful update
      setPassword("");
    } catch (err) {
      console.error("Error updating Kavita settings:", err);
      if (err instanceof Error) {
        showToast(err.message, 'error');
      } else {
        showToast("An unexpected error occurred", 'error');
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
          <div className="mb-6">
            <label className="block mb-2 text-xl">Kavita Server URL:</label>
            <input
              type="text"
              value={serverIP}
              onChange={(e) => setServerIP(e.target.value)}
              className="w-full p-4 bg-zinc-800 border border-zinc-700 rounded text-white text-lg"
              placeholder="e.g. 192.168.1.100:5001"
            />
            <p className="text-gray-400 mt-1 text-sm">The URL of your Kavita server, including port number</p>
          </div>
          
          <div className="mb-6">
            <label className="block mb-2 text-xl">Kavita Username:</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              className="w-full p-4 bg-zinc-800 border border-zinc-700 rounded text-white text-lg"
              placeholder="Your Kavita username"
            />
          </div>
          
          <div className="mb-6">
            <label className="block mb-2 text-xl">Kavita Password:</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="w-full p-4 bg-zinc-800 border border-zinc-700 rounded text-white text-lg"
              placeholder={hasStoredPassword ? "Leave empty to keep current password" : "Enter your Kavita password"}
            />
            {hasStoredPassword && (
              <p className="text-gray-400 mt-1 text-sm">Password is stored. Leave empty to keep current password.</p>
            )}
          </div>
          
          <div className="mb-8">
            <label className="block mb-2 text-xl">Kavita API Key:</label>
            <input
              type="text"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              className="w-full p-4 bg-zinc-800 border border-zinc-700 rounded text-white text-lg"
              placeholder="Enter your Kavita API key"
            />
            <p className="text-gray-400 mt-1 text-sm">API key can be found in your Kavita user settings</p>
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