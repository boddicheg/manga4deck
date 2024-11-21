import React, { useEffect, useState } from 'react';

const App: React.FC = () => {
  const [message, setMessage] = useState<string>('');

  useEffect(() => {
    fetch('http://localhost:1337/api/status')
      .then((response) => response.json())
      .then((data) => {
        setMessage(data.status);
      });
  }, []);

  return (
    <div>
      <h1>server status: {message}</h1>
    </div>
  );
};

export default App;

