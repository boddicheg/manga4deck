import React, { useState, useEffect, createContext, useContext, useCallback } from 'react';
import Toast, { ToastType } from './Toast';

export interface ToastMessage {
  id: string;
  message: string;
  type: ToastType;
  duration?: number;
  visible: boolean; // Track visibility state for animation
  timestamp?: number; // Add timestamp for tracking when the toast was created
}

interface ToastContextProps {
  showToast: (message: string, type: ToastType, duration?: number) => void;
  hideToast: (id: string) => void;
}

const ToastContext = createContext<ToastContextProps | undefined>(undefined);

export const useToast = () => {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error('useToast must be used within a ToastProvider');
  }
  return context;
};

export const ToastProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [toasts, setToasts] = useState<ToastMessage[]>([]);

  const showToast = useCallback((message: string, type: ToastType, duration = 3000) => {
    const id = Math.random().toString(36).substring(2, 9);
    setToasts((prevToasts) => [...prevToasts, { 
      id, 
      message, 
      type, 
      duration, 
      visible: true,
      timestamp: Date.now()
    }]);
  }, []);

  const hideToast = useCallback((id: string) => {
    // First set visible to false to trigger animation
    setToasts((prevToasts) => 
      prevToasts.map(toast => 
        toast.id === id ? { ...toast, visible: false } : toast
      )
    );
    
    // Then remove the toast after animation completes
    setTimeout(() => {
      setToasts((prevToasts) => prevToasts.filter((toast) => toast.id !== id));
    }, 500); // Slightly longer than animation duration to ensure completion
  }, []);

  // Auto-close toasts after their duration
  useEffect(() => {
    // Set up an interval to check for toasts that need to be closed
    const autoCloseInterval = setInterval(() => {
      const currentTime = Date.now();
      
      toasts.forEach(toast => {
        if (toast.visible && toast.timestamp && toast.duration) {
          const timeElapsed = currentTime - toast.timestamp;
          
          // If the toast has been visible for longer than its duration, hide it
          if (timeElapsed >= toast.duration) {
            hideToast(toast.id);
          }
        }
      });
    }, 500); // Check every 500ms instead of every second
    
    return () => clearInterval(autoCloseInterval);
  }, [toasts, hideToast]);

  // Clean up toasts that have been invisible for too long (safety cleanup)
  useEffect(() => {
    const cleanupInterval = setInterval(() => {
      setToasts(prevToasts => {
        const now = Date.now();
        return prevToasts.filter(toast => {
          // Keep all visible toasts
          if (toast.visible) return true;
          
          // If toast has been invisible for more than 1 second, remove it
          const toastAge = now - (toast.timestamp || now);
          return toastAge < 1000;
        });
      });
    }, 2000);
    
    return () => clearInterval(cleanupInterval);
  }, []);

  return (
    <ToastContext.Provider value={{ showToast, hideToast }}>
      {children}
      <ToastContainer toasts={toasts} hideToast={hideToast} />
    </ToastContext.Provider>
  );
};

interface ToastContainerProps {
  toasts: ToastMessage[];
  hideToast: (id: string) => void;
}

const ToastContainer: React.FC<ToastContainerProps> = ({ toasts, hideToast }) => {
  return (
    <div className="fixed bottom-4 right-4 flex flex-col-reverse space-y-reverse space-y-4 z-50">
      {toasts.map((toast) => (
        <Toast
          key={toast.id}
          message={toast.message}
          type={toast.type}
          duration={toast.duration}
          onClose={() => hideToast(toast.id)}
          isVisible={toast.visible}
        />
      ))}
    </div>
  );
};

export default ToastContainer; 