import "./App.css";

import { useState } from "react";

import { emit } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/api/dialog';

function App() {

  const [selectedFile, setSelectedFile] = useState<string | null>(null);

  async function openFileSelectDialog() {
    const selected = await open({
      title: 'Select markdown file.',
      multiple: false,
      filters: [{
        name: 'markdown',
        extensions: ['md', 'markdown']
      }]
    });

    if (typeof selected === 'string') {
      setSelectedFile(selected);
      emit('stop_watch', {});
      emit('start_watch', selected);
    }
  }

  return (
    <div className="container">
      <h1>Welcome to Simai!</h1>
      <label>
        Selected file: {selectedFile}
        <button onClick={(_) => { openFileSelectDialog() }}>select file</button>
      </label>
    </div>
  );
}

export default App;
