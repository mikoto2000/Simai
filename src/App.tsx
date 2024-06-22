import "./App.css";

import { useEffect, useState } from "react";
import { emit, listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/api/dialog';
import { appWindow } from '@tauri-apps/api/window';

import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

function App() {

  const CUSTOM_CSS_KEY = "custom.css";

  const [selectedMdFile, setSelectedMdFile] = useState<string | null>(null);
  const [mdContent, setMdContent] = useState<string>("");

  const [selectedCssFile, setSelectedCssFile] = useState<string | null>(null);
  const [cssContent, setCssContent] = useState<string | undefined>(undefined);

  useEffect(() => {
    (async () => {
      // ファイルのドロップを購読
      appWindow.onFileDropEvent(async (event) => {
        if (event.payload.type === 'hover') {
          console.log('User hovering', event);
        } else if (event.payload.type === 'drop') {
          console.log('User dropped', event);
          const filePath = event.payload.paths[0];
          if (filePath.endsWith(".css")) {
            setSelectedCssFile(filePath);
            emit('stop_watch_md', {});
            emit('start_watch_md', { path: filePath });
          } else {
            setSelectedMdFile(filePath);
            emit('stop_watch_css', {});
            emit('start_watch_css', { path: filePath });
          }
        } else {
          console.log('File drop cancelled');
        }
      });

      // md ファイル更新イベントを購読
      listen('update_md', (event: any) => {
        console.log(event);
        setMdContent(event.payload);
      });

      // css ファイル更新イベントを購読
      listen('update_css', (event: any) => {
        console.log(event);
        setCssContent(event.payload);
      });
    })();
  }, []);

  async function openMdFileSelectDialog() {
    const selected = await open({
      title: 'Select markdown file.',
      multiple: false,
      filters: [{
        name: 'markdown',
        extensions: ['md', 'markdown']
      }]
    });

    if (typeof selected === 'string') {
      setSelectedMdFile(selected);
      emit('stop_watch_md', {});
      emit('start_watch_md', { path: selected });

    }
  }

  async function openCssFileSelectDialog() {
    const selected = await open({
      title: 'Select markdown file.',
      multiple: false,
      filters: [{
        name: 'css',
        extensions: ['css']
      }]
    });

    if (typeof selected === 'string') {
      setSelectedCssFile(selected);
      emit('stop_watch_css', {});
      emit('start_watch_css', { path: selected });

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
          Markdown file: {selectedMdFile}
          <button onClick={(_) => { openMdFileSelectDialog() }}>select md file</button>
        </label>
        <label>
          Css file: {selectedCssFile}
          <button onClick={(_) => { openCssFileSelectDialog() }}>select css file</button>
        </label>
      </div>
      <div>
        <ReactMarkdown remarkPlugins={[remarkGfm]} children={mdContent} />
      </div>
    </>
  );
}

export default App;
