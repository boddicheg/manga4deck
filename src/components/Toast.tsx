import React, { useEffect, useState } from 'react';

export type ToastType = 'success' | 'error' | 'info' | 'warning';

export interface ToastProps {
  message: string;
  type: ToastType;
  duration?: number;
  onClose?: () => void;
  isVisible: boolean;
}

const Toast: React.FC<ToastProps> = ({
  message,
  type,
  duration = 3000,
  onClose,
  isVisible
}) => {
  const [visible, setVisible] = useState(false);
  const [shouldRender, setShouldRender] = useState(false);

  // Handle the animation and visibility states
  useEffect(() => {
    if (isVisible) {
      setShouldRender(true);
      // Small delay to ensure the DOM is updated before starting the animation
      setTimeout(() => setVisible(true), 10);
    } else {
      setVisible(false);
      // Wait for the animation to complete before removing from DOM
      const timer = setTimeout(() => setShouldRender(false), 300);
      return () => clearTimeout(timer);
    }
  }, [isVisible]);

  // Handle the auto-close timer - only needed if we want the component to self-close
  // This is now handled by the ToastContainer
  // useEffect(() => {
  //   // Only set the timer if the toast is visible and duration is positive
  //   if (isVisible && duration > 0) {
  //     const timer = setTimeout(() => {
  //       if (onClose) {
  //         onClose();
  //       }
  //     }, duration);
      
  //     // Clean up the timer when the component unmounts or dependencies change
  //     return () => clearTimeout(timer);
  //   }
  // // Include all dependencies used inside the effect
  // }, [duration, isVisible, onClose]);

  if (!shouldRender) return null;

  const getToastClasses = (): string => {
    const baseClasses = "min-w-[300px] max-w-md px-6 py-3 rounded-md shadow-lg transition-all duration-300";
    const animationClasses = visible 
      ? "translate-x-0 opacity-100" 
      : "translate-x-full opacity-0";
    
    let typeClasses = "";
    switch (type) {
      case 'success':
        typeClasses = "bg-green-500 text-white";
        break;
      case 'error':
        typeClasses = "bg-red-500 text-white";
        break;
      case 'warning':
        typeClasses = "bg-yellow-500 text-white";
        break;
      case 'info':
      default:
        typeClasses = "bg-blue-500 text-white";
        break;
    }
    
    return `${baseClasses} ${typeClasses} ${animationClasses}`;
  };

  const getIcon = (): JSX.Element => {
    switch (type) {
      case 'success':
        return (
          <svg className="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
          </svg>
        );
      case 'error':
        return (
          <svg className="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        );
      case 'warning':
        return (
          <svg className="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
        );
      case 'info':
      default:
        return (
          <svg className="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        );
    }
  };

  const handleClose = () => {
    setVisible(false);
    // Wait for the animation to complete before calling onClose
    setTimeout(() => {
      if (onClose) onClose();
    }, 300);
  };

  return (
    <div className={getToastClasses()}>
      <div className="flex items-center">
        {getIcon()}
        <span className="text-sm font-medium">{message}</span>
        <button
          onClick={handleClose}
          className="ml-4 text-white hover:text-gray-200 focus:outline-none"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  );
};

export default Toast; 