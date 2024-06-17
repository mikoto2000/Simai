import "./App.css";

import { useEffect, useState } from "react";
import { emit, listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/api/dialog';

import { Store } from "tauri-plugin-store-api";

import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

function App() {

  const CUSTOM_CSS_KEY = "custom.css";

  const [selectedFile, setSelectedFile] = useState<string | null>(null);

  const [cssContent, setCssContent] = useState<string | undefined>(undefined);

  const [content, setContent] = useState<string>("");

  const [store, setStore] = useState<Store | null>(null);

  useEffect(() => {
    (async () => {
      const store = new Store(CUSTOM_CSS_KEY);
      setStore(store);
      const userCss = await store.get<string>(CUSTOM_CSS_KEY);
      if (userCss) {
        setCssContent(userCss);
      }

      listen('update_md', (event: any) => {
        console.log(event);
        setContent(event.payload);
      });
    })();
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
        if (store) {
          store.set(CUSTOM_CSS_KEY, content);
        }
      };
      reader.readAsText(file);
    }
  }

  return (
    <>
      <style>
        {cssContent ? cssContent : ""}
      </style>
      <div className="container">
        <h1>Welcome to Simai!</h1>
        <label>
          Selected file: {selectedFile}
          <button onClick={(_) => { openFileSelectDialog() }}>select file</button>
        </label>
        <p>
          <input type="file" onChange={applyCss} accept=".css" />
        </p>
      </div>
      <div>
        <ReactMarkdown remarkPlugins={[remarkGfm]} children={content} />
      </div>
    </>
  );
}

export default App;
