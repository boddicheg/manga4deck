import React, { useEffect, useState, useRef } from "react";
import { fetchClearCache, fetchServerStatus, fetchUpdateLibrary } from "../services/Api";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";

const Dashboard: React.FC = () => {

  const [serverStatus, setServerStatus] = useState<boolean>(false);
  const [logged, setLogged] = useState<string>("");
  const [serverIP, setServerIP] = useState<string>("");
  const [cacheSize, setCacheSize] = useState<number>(0.0);
  const divRefs = useRef<(HTMLDivElement | null)[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const currentIndexRef = useRef(currentIndex);
  const [loading, setLoading] = useState<boolean>(true);
  const [, setError] = useState<string | null>(null);
  const [command, setCommand] = useState<string | null>(null);
  const navigate = useNavigate();
  const fetchInterval = 5000; // Fetch every 5 seconds

  async function exitApp() {
    await invoke("exit_app", {});
  }

  const navigateTo = (uri: string | null | undefined) => {
    if (uri)
      navigate(`${uri}`);
  };

  const cycleFocus = (direction: "next" | "prev") => {
    const nextIndex =
      direction === "next"
        ? currentIndexRef.current + 1 >= 5
          ? 5 - 1
          : currentIndexRef.current + 1
        : currentIndexRef.current - 1 < 0
          ? 0
          : currentIndexRef.current - 1;

    setCurrentIndex(nextIndex);
    divRefs.current[nextIndex]?.focus();
  };

  const enterDirectory = () => {
    const currentDiv = divRefs.current[currentIndexRef.current];
    const route = currentDiv?.getAttribute("data-route");
    console.log(route);
    const exit_ = async () => {
      await exitApp();
    };
    if (route == "/exit-app") exit_();
    if (route == "/clean-cache") {
      setCommand("clean-cache")
    }
    else if (route == "/update-lib") {
      setCommand("update-lib")
    }
    else navigateTo(route);
  };

  const handleKey: (this: Window, ev: KeyboardEvent) => any = function (
    this: Window,
    event: KeyboardEvent
  ) {
    switch (event.key) {
      case "ArrowUp":
        console.log("ArrowUp");
        break;
      case "ArrowDown":
        console.log("ArrowDown");
        break;
      case "ArrowLeft":
        console.log("ArrowLeft");
        cycleFocus("prev");
        break;
      case "ArrowRight":
        console.log("ArrowRight");
        cycleFocus("next");
        break;
      case "Enter":
        console.log("Enter");
        enterDirectory();
        break;
      case "Backspace":
        console.log("Backspace");
        break;
      default:
        console.log(`Key pressed: ${event.key}`);
    }
  };

  const updateLib = async () => {
    await fetchUpdateLibrary();
  }

  const clearCache = async () => {
    await fetchClearCache();
  }

  const getServerStatus = async () => {
    setLoading(true);
    try {
      const data = await fetchServerStatus();
      setServerStatus(data.status);
      setLogged(data.logged_as);
      setServerIP(data.ip)
      setLoading(false);
      setCacheSize(data.cache)
    } catch (err) {
      setServerStatus(false);
      setLogged("");
      setServerIP("");
      setCacheSize(0.0)
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError("An unexpected error occurred");
      }
    }
  };

  useEffect(() => {
    getServerStatus();
    // Key press events
    window.addEventListener("keydown", handleKey);
    const intervalId = setInterval(getServerStatus, fetchInterval);
    return () => {
      window.removeEventListener("keydown", handleKey); // Clean up
      clearInterval(intervalId);
    };
  }, []);

  useEffect(() => {
    currentIndexRef.current = currentIndex;
  }, [currentIndex]);

  useEffect(() => {
    console.log(command)
    if (command == "update-lib") updateLib();
    if (command == "clean-cache") clearCache();
  }, [command]);

  return (
    <div className="w-full h-screen p-4 bg-zinc-900">
      <div className={
        (serverStatus || !loading) ? "bg-green-500 w-full text-center pt-1 pb-1 text-white"
          : "bg-red-500 w-full text-center pt-1 pb-1 text-white"
      }>

        {
          loading ? "Loading..." :
            ("Server status: " + (serverStatus ? "online" : "offline")
              + (serverStatus ? ", logged as " + logged : "")
              + (serverStatus ? ", ip is " + serverIP : ""))
        }
      </div>

      <div className="grid grid-cols-8 gap-4">
        <div
          key={0}
          data-route={"/shelf"}
          onClick={() => navigateTo("/shelf")}
          ref={(el) => (divRefs.current[0] = el)} // Assign ref
          tabIndex={-1} // Make it focusable but not in tab order
          className={`
            border-2 
            rounded 
            bg-gray-300
            min-h-[200px]
            min-w-[150px]
            m-3
            inline-block
            ${currentIndex === 0
              ? "border-2 border-red-500 "
              : "border-gray-300"
            }`}
        >
          <div className="text-lg text-center font-bold mt-12">Kavita</div>
          <div className="text-sm text-gray-600 text-center">{serverIP}</div>
        </div>

        <div
          key={1}
          data-route={"/"}
          onClick={() => navigateTo("/")}
          ref={(el) => (divRefs.current[1] = el)} // Assign ref
          tabIndex={-1} // Make it focusable but not in tab order
          className={`
            border-2 
            rounded 
            bg-gray-300
            min-h-[200px]
            min-w-[150px]
            m-3
            inline-block
            ${currentIndex === 1
              ? "border-2 border-red-500 "
              : "border-gray-300"
            }`}
        >
          <div className="text-lg text-center font-bold mt-12">Clean Cache</div>
          <div className="text-sm text-gray-600 text-center">{cacheSize.toFixed(2)}Gb</div>
        </div>

        <div
          key={2}
          data-route={"/update-lib"}
          ref={(el) => (divRefs.current[2] = el)} // Assign ref
          tabIndex={-1} // Make it focusable but not in tab order
          className={`
            border-2 
            rounded 
            bg-gray-300
            min-h-[200px]
            min-w-[150px]
            m-3
            inline-block
            ${currentIndex === 2
              ? "border-2 border-red-500 "
              : "border-gray-300"
            }`}
        >
          <div className="text-lg text-center font-bold mt-12">Update</div>
          <div className="text-sm text-gray-600 text-center">Server Kavita</div>
        </div>

        <div
          key={3}
          data-route={"/"}
          onClick={() => navigateTo("/")}
          ref={(el) => (divRefs.current[3] = el)} // Assign ref
          tabIndex={-1} // Make it focusable but not in tab order
          className={`
            border-2 
            rounded 
            bg-gray-300
            min-h-[200px]
            min-w-[150px]
            m-3
            inline-block
            ${currentIndex === 3
              ? "border-2 border-red-500"
              : "border-gray-300"
            }`}
        >
          <div className="text-lg text-center font-bold mt-12">Exit</div>
          <div className="text-sm text-gray-600 text-center">Close app</div>
        </div>
      </div>
    </div>
  );
};

export default Dashboard;
