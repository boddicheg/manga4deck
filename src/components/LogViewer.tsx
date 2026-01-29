import React, { useEffect, useState, useRef } from "react";

interface LogViewerProps {
  isOpen: boolean;
  onClose: () => void;
}

const LogViewer: React.FC<LogViewerProps> = ({ isOpen, onClose }) => {
  const [logs, setLogs] = useState<string[]>([]);
  const logEndRef = useRef<HTMLDivElement>(null);
  const [isPolling, setIsPolling] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [filter, setFilter] = useState<string>("");

  const fetchLogs = async () => {
    try {
      setLoading(true);
      setError(null);
      
      // Add a timestamp to avoid caching
      const timestamp = new Date().getTime();
      const response = await fetch(`http://localhost:11337/api/logs?t=${timestamp}`);
      
      if (response.ok) {
        const data = await response.json();
        
        if (data.logs) {
          setLogs(data.logs);
        } else {
          setLogs([]);
        }
      } else {
        setError(`Failed to fetch logs: ${response.statusText}`);
      }
    } catch (error) {
      setError(`Error fetching logs: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      fetchLogs();
      
      if (isPolling) {
        const interval = setInterval(fetchLogs, 2000); // Poll every 2 seconds
        return () => clearInterval(interval);
      }
    }
  }, [isOpen, isPolling]);

  useEffect(() => {
    // Scroll to bottom when logs update
    if (logEndRef.current) {
      logEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs]);

  if (!isOpen) return null;

  const getLogClass = (log: string) => {
    if (log.includes('[ERROR]')) return 'text-red-400';
    if (log.includes('[WARNING]')) return 'text-yellow-400';
    if (log.includes('[INFO]')) return 'text-blue-400';
    if (log.includes('[DEBUG]')) return 'text-green-400';
    return 'text-gray-300';
  };

  const filteredLogs = filter 
    ? logs.filter(log => log.toLowerCase().includes(filter.toLowerCase()))
    : logs;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-zinc-900 w-3/4 h-3/4 rounded-lg shadow-lg flex flex-col">
        <div className="flex justify-between items-center p-4 border-b border-zinc-700">
          <h2 className="text-xl text-white font-bold">Backend Logs</h2>
          <div className="flex items-center space-x-4">
            <input
              type="text"
              placeholder="Filter logs..."
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="bg-zinc-800 text-white px-2 py-1 rounded border border-zinc-700"
            />
            <label className="text-white flex items-center">
              <input
                type="checkbox"
                checked={isPolling}
                onChange={() => setIsPolling(!isPolling)}
                className="mr-2"
              />
              Auto-refresh
            </label>
            <button
              onClick={fetchLogs}
              className="bg-blue-600 hover:bg-blue-500 text-white py-1 px-3 rounded"
            >
              Refresh
            </button>
            <button
              onClick={onClose}
              className="bg-red-600 hover:bg-red-500 text-white py-1 px-3 rounded"
            >
              Close
            </button>
          </div>
        </div>
        <div className="flex-1 overflow-auto p-4 bg-zinc-800 text-white font-mono text-sm">
          {loading && logs.length === 0 ? (
            <div className="text-blue-400 text-center mt-4">Loading logs...</div>
          ) : error ? (
            <div className="text-red-400 text-center mt-4">Error: {error}</div>
          ) : filteredLogs.length === 0 ? (
            <div className="text-gray-400 text-center mt-4">
              {filter ? "No logs match your filter" : "No logs available"}
            </div>
          ) : (
            <>
              <div className="mb-2 text-gray-400">
                Showing {filteredLogs.length} of {logs.length} logs
                {filter && ` (filtered by "${filter}")`}
              </div>
              {filteredLogs.map((log, index) => (
                <div key={index} className={`mb-1 ${getLogClass(log)}`}>
                  {log}
                </div>
              ))}
            </>
          )}
          <div ref={logEndRef} />
        </div>
      </div>
    </div>
  );
};

export default LogViewer; 