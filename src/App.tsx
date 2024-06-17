import "./App.css";

import { useEffect, useState } from "react";

import { emit, listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/api/dialog';

import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

function App() {

  const [selectedFile, setSelectedFile] = useState<string | null>(null);

  const [content, setContent] = useState<string>("");

  useEffect(() => {
    listen('update_md', (event: any) => {
      console.log(event);
      setContent(event.payload);
    });
  }, []);

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

  async function applyCss(event: any) {
    console.log(event);

    // ファイル取得
    const file = event.target.files[0];

    // ファイル読み込み
    if (file) {
      const reader = new FileReader();
      reader.onload = function(e) {
        const content = e.target?.result?.toString();
        setCssContent(content);
      };
      reader.readAsText(file);
    }
  }

  return (
    <>
      <div className="container">
        <h1>Welcome to Simai!</h1>
        <label>
          Selected file: {selectedFile}
          <button onClick={(_) => { openFileSelectDialog() }}>select file</button>
        </label>
      </div>
      <div>
        <ReactMarkdown remarkPlugins={[remarkGfm]} children={content} />
      </div>
    </>
  );
}

export default App;
