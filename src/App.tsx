import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";
import Button from '@mui/material/Button';
import TextField from "@mui/material/TextField";
import Box from "@mui/material/Box";
import Paper from "@mui/material/Paper";
import { DataGrid, GridColDef } from '@mui/x-data-grid';


function App() {

  const obj: {[index: string]:any} = {}
  const [passwords, updatePasswordsList] = useState(obj);
  const [name, setName] = useState("");
  const columns: GridColDef[] = [
    {
      field: 'id',
      headerName: 'Name',
      width: 150,
    },
  ];


  async function getPasswords() {
    const data: object = await invoke("get_passwords")
    updatePasswordsList(data);
  }

  function renderPasswords() {
    const rows = Object.keys(passwords).map((keyName: string) => {return {id: keyName}});
    console.log("hello", rows);

    return (
      <>
      <Box sx={{ height: 400, width: '100%' }}>
      <DataGrid
        rows={rows}
        columns={columns}
        initialState={{
          pagination: {
            paginationModel: {
              pageSize: 5,
            },
          },
        }}
        pageSizeOptions={[5]}
        checkboxSelection
        disableRowSelectionOnClick
      />
    </Box>
    </>
    );
  }

  return (
    <div className="container">
      <h1>Vikeypass</h1>

      <Paper>

        <form
          className="row"
          onSubmit={(e) => {
            e.preventDefault();
            getPasswords()
          }}
        >
          <TextField
            id="greet-input"
            onChange={(e) => setName(e.currentTarget.value)}
            placeholder="Enter a password..."
          />
          <Button type="submit" variant="contained">Find</Button>
        </form>

        <p>{renderPasswords()}</p>
      </Paper>
    </div>
  );
}

export default App;
