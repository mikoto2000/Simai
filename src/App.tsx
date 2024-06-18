import "./App.css";

import { useEffect, useState } from "react";
import { emit, listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/api/dialog';
import { appWindow } from '@tauri-apps/api/window';

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
      // ファイルのドロップを購読
      appWindow.onFileDropEvent((event) => {
        if (event.payload.type === 'hover') {
          // TODO: ドロップできそうな表示に変更
          console.log('User hovering', event.payload.paths);
        } else if (event.payload.type === 'drop') {
          console.log('User dropped', event.payload.paths);
          const filePath = event.payload.paths[0];
          setSelectedFile(filePath);
          emit('stop_watch', {});
          emit('start_watch', { path: filePath });
        } else {
          console.log('File drop cancelled');
        }
      });

      // カスタム CSS 読み込み
      const store = new Store(CUSTOM_CSS_KEY);
      setStore(store);
      const userCss = await store.get<string>(CUSTOM_CSS_KEY);
      if (userCss) {
        setCssContent(userCss);
      }

      // ファイル更新イベントを購読
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
      emit('start_watch', { path: selected });

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
      <style>
        {`strong.simai { font-size: 1.175em }`}
      </style>
      <div className="container">
        <h1><strong className="simai">Si</strong>mple <strong className="simai">Ma</strong>rkdown prev<strong className="simai">i</strong>ewer</h1>
        <label>
          Markdown file: {selectedFile}
          <button onClick={(_) => { openFileSelectDialog() }}>select file</button>
        </label>
        <p>
          <label>Custom css file: <input type="file" onChange={applyCss} accept='.css' /></label>
        </p>
      </div>
      <div>
        <ReactMarkdown remarkPlugins={[remarkGfm]} children={content} />
      </div>
    </>
  );
}

export default App;
