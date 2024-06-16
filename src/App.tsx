import "./App.css";

import { emit } from '@tauri-apps/api/event'

function App() {
  function emitSelectedWatch() {
    emit('start_watch', "../README.md");
  }

  function emitStop() {
    emit('stop_watch', {});
  }

  function emitSelectedFile() {
    emit('selected_file', "../README.md");
  }

  return (
    <div className="container">
      <h1>Welcome to Simai!</h1>
      <button onClick={(_) => {emitSelectedWatch()}}>start watch</button>
      <button onClick={(_) => {emitStop()}}>stop</button>
      <button onClick={(_) => {emitSelectedFile()}}>selected file</button>
    </div>
  );
}

export default App;
