# %%
import requests
import json
from pathlib import Path

workspace_dir = Path(__file__).parent

# %%
lab_url = "http://localhost:8888/"
with open(".secret", "r") as f:
  token = f.read().strip()

# %%
client = requests.Session()
resp = client.get(f"{lab_url}?token={token}")
resp.cookies

# %%
def try_save(resp, filename: Path):
  if resp.status_code == 200:
    try:
      data = resp.json()
      with open(workspace_dir / f"samples/{filename}.json", "w") as f:
        json.dump(data, f, indent=2)
    except json.JSONDecodeError as e:
      print(f"JSON decode error: {e}")
  else:
    print(f"Error: {resp.status_code}")

# %%
resp = client.get(f"{lab_url}/api/")
try_save(resp, Path("[GET]__root"))

# %%
resp = client.get(f"{lab_url}/lab/api/workspaces")
try_save(resp, Path("[GET]lab__workspaces"))

# %%
for i in ["sessions", "kernels", "contents", "terminals", "kernelspecs", "status", "me"]:
  resp = client.get(f"{lab_url}/api/{i}")
  try_save(resp, Path(f"[GET]{i}"))

# %%
import requests
byte_range = range(2)
resp = requests.get(f"{lab_url}/files/hello.txt?token={token}", headers={
  "Range": f"bytes={byte_range.start}-{byte_range.stop - 1}"
})
resp.text
# %%
