import "./App.css";

import { open } from '@tauri-apps/api/dialog';
import { emit, listen } from '@tauri-apps/api/event';
import { appWindow } from '@tauri-apps/api/window';
import { useEffect, useState } from "react";
import { Store } from "tauri-plugin-store-api";

import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeRaw from 'rehype-raw';
import remarkToc from 'remark-toc'
import matter from 'gray-matter';

import { Buffer } from "buffer";
window.Buffer = Buffer;

function App() {

  const CUSTOM_CSS_KEY = "custom.css";

  const [selectedMdFile, setSelectedMdFile] = useState<string | null>(null);
  const [metadata, setMetadata] = useState<any>(undefined);
  const [mdContent, setMdContent] = useState<string>("");

  const [selectedCssFile, setSelectedCssFile] = useState<string | null>(null);
  const [cssContent, setCssContent] = useState<string | undefined>(undefined);

  const [_, setStore] = useState<Store | null>(null);

  useEffect(() => {
    (async () => {
      // ファイルのドロップを購読
      const unlisten = await appWindow.onFileDropEvent(async (event) => {
        if (event.payload.type === 'hover') {
          console.log('User hovering', event);
        } else if (event.payload.type === 'drop') {
          console.log('User dropped', event);
          event.payload.paths.forEach(async (filePath) => {
            console.log(filePath);
            if (filePath.endsWith(".css")) {
              setSelectedCssFile((_) => filePath);
              await emit('stop_watch_css', {});
              await emit('start_watch_css', { path: filePath });
            } else {
              setSelectedMdFile((_) => filePath);
              await emit('stop_watch_md', {});
              await emit('start_watch_md', { path: filePath });
            }
          });
        } else {
          console.log('File drop cancelled');
        }
      });

      // 前回の CSS の内容を読み込み
      const store = new Store(CUSTOM_CSS_KEY);
      const userCss = await store.get<any>(CUSTOM_CSS_KEY);
      if (userCss) {
        console.log(userCss)
        setSelectedCssFile((_) =>
          userCss?.content?.path
            ?
            userCss?.content?.path + "(Previous cache)"
            :
            ""
        );
        setCssContent((_) => userCss?.content?.content ?? "");
      }
      setStore((_) => store);

      // md ファイル更新イベントを購読
      listen('update_md', (event: any) => {
        console.log(event);
        const { data, content } = matter(event.payload.content);
        setSelectedMdFile((_) => event.payload.path);
        setMetadata(data);
        setMdContent((_) => content);
      });

      // css ファイル更新イベントを購読
      listen('update_css', (event: any) => {
        console.log(event);
        setSelectedCssFile((_) => event.payload.path);
        setCssContent((_) => event.payload.content);
        if (store) {
          store.set(CUSTOM_CSS_KEY,
            {
              path: selectedCssFile,
              content: event.payload
            }
          );
        }
      });
      return () => {
        unlisten();
      }
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
      console.log(selected);
      setSelectedMdFile((_) => selected);
      await emit('stop_watch_md', {});
      await emit('start_watch_md', { path: selected });

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
      console.log(selected);
      setSelectedCssFile((_) => selected);
      await emit('stop_watch_css', {});
      await emit('start_watch_css', { path: selected });

    }
  }

  return (
    <>
      <div className="container">
        <label>
          Markdown file: {selectedMdFile}
          <button onClick={(_) => { openMdFileSelectDialog() }}>
            select md file
          </button>
        </label>
        <label>
          Css file: {selectedCssFile}
          <button onClick={(_) => { openCssFileSelectDialog() }}>
            select css file
          </button>
        </label>
      </div>
      <div>
        <table>
          <thead>
            <tr>
              {Object.keys(metadata).map((key) => (
                <th key={key}>{key}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            <tr>
              {Object.values(metadata).map((value, index) => (
                <td key={index}>{value as any}</td>
              ))}
            </tr>
          </tbody>
        </table>
        <ReactMarkdown
          remarkPlugins={
            [
              remarkGfm,
              [remarkToc, { heading: '目次' }]
            ]
          }
          rehypePlugins={[rehypeRaw]}
          children={
            "# 目次\n\n" + mdContent
          } />
      </div>
      <style>
        {cssContent ? cssContent : ""}
      </style>
    </>
  );
}

export default App;
