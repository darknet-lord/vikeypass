import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

function App() {
  const obj: {[index: string]:any} = {}
  const [passwords, updatePasswordsList] = useState(obj);
  const [name, setName] = useState("");

  async function getPasswords() {
    const data: object = await invoke("get_passwords")
    updatePasswordsList(data);
  }

  function renderPasswords() {
    return Object.keys(passwords).map((keyName: string, i: number) => (
      <li className="travelcompany-input" key={i}>
          <span className="input-label">key: {i} Name: {passwords[keyName]}</span>
      </li>
    ))
  }

  return (
    <div className="container">
      <h1>Vikeypass</h1>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          getPasswords()
        }}
      >
        <input
          id="greet-input"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a password..."
        />
        <button type="submit">Find</button>
      </form>

      <p>{renderPasswords()}</p>
    </div>
  );
}

export default App;
