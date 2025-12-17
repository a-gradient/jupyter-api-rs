from pathlib import Path
from jupyterlab.labapp import LabApp

workspace_dir = Path(__file__).parent

app = LabApp.initialize_server([
  "--notebook-dir=" + str(workspace_dir/"tmp"),
  "--no-browser",
])
with open(workspace_dir/".secret", "w") as f:
  f.write(app.token)
app.start()
