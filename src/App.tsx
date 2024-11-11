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

declare global {
  interface Window {
    Buffer: typeof import('buffer').Buffer;
  }
}

import { Buffer } from "buffer";
import { writeText } from "@tauri-apps/api/clipboard";
window.Buffer = Buffer;

function App() {

  const CUSTOM_CSS_KEY = "custom.css";
  const TCP_CONFIG_KEY = "tcp.config";

  const [selectedMdFile, setSelectedMdFile] = useState<string | null>(null);
  const [metadata, setMetadata] = useState<any>(undefined);
  const [mdContent, setMdContent] = useState<string>("");

  const [selectedCssFile, setSelectedCssFile] = useState<string | null>(null);
  const [cssContent, setCssContent] = useState<string | undefined>(undefined);
  const [isCache, setIsCache] = useState<boolean>(true);

  const [store, setStore] = useState<Store | null>(null);

  const [isTcpListener, setIsTcpListener] = useState<boolean>(false);
  const [tcpAddress, setTcpAddress] = useState<string>("127.0.0.1");
  const [tcpPort, setTcpPort] = useState<number>(7878);

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
              setIsCache(false);
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
            userCss?.content?.path
            :
            ""
        );
        setCssContent((_) => userCss?.content?.content ?? "");
      }

      // 前回の TCP 設定を読み込み
      const tcpConfig = await store.get<any>(TCP_CONFIG_KEY);
      if (tcpConfig) {
        setTcpAddress(tcpConfig.address);
        setTcpPort(tcpConfig.port);
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
        setIsCache(false);
        if (store) {
          store.set(CUSTOM_CSS_KEY,
            {
              path: selectedCssFile,
              content: event.payload
            }
          );
        }
      });

      // Listen for the new event emitted by the TCP server
      listen('update_md_tcp', (event: any) => {
        console.log(event);
        const { data, content } = matter(event.payload.content);
        setMetadata(data);
        setMdContent((_) => content);
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
      setIsCache(false);
      await emit('stop_watch_css', {});
      await emit('start_watch_css', { path: selected });

    }
  }

  const handleToggleChange = async () => {
    setIsTcpListener(!isTcpListener);
    if (isTcpListener) {
      await emit('stop_tcp_listener', {});
    } else {
      await emit('start_tcp_listener', { address: tcpAddress, port: tcpPort });
      if (store) {
        store.set(TCP_CONFIG_KEY, { address: tcpAddress, port: tcpPort });
        await store.save();
      }
    }
  };

  return (
    <>
      <div className="container">
        <div>
          <label>
            Markdown file: {selectedMdFile}
            <button onClick={(_) => { openMdFileSelectDialog() }}>
              select md file
            </button>
          </label>
          <button onClick={(_) => {
            if (selectedMdFile) {
              writeText(selectedMdFile)
            }
          }}>
            copy md file path
          </button>
        </div>
        <div>
          <label>
            Css file: {selectedCssFile}
            {
              isCache
                ?
                " (Previous cache)"
                :
                ""
            }
            <button onClick={(_) => { openCssFileSelectDialog() }}>
              select css file
            </button>
          </label>
          <button onClick={(_) => {
            if (selectedCssFile) {
              writeText(selectedCssFile)
            }
          }}>
            copy css file path
          </button>
        </div>
        <div>
          <label>
            TCP Listener
            <input
              type="checkbox"
              checked={isTcpListener}
              onChange={handleToggleChange}
            />
          </label>
          <div>
            <label>
              Address:
              <input
                type="text"
                value={tcpAddress}
                onChange={(e) => setTcpAddress(e.target.value)}
              />
            </label>
            <label>
              Port:
              <input
                type="number"
                value={tcpPort}
                onChange={(e) => setTcpPort(Number(e.target.value))}
              />
            </label>
          </div>
        </div>
      </div>
      <div>
        {
          metadata ?
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
            :
            <></>
        }
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
