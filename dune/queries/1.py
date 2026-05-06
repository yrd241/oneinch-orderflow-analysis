Bash(python3 << 'EOF'
      import urllib.request, json

      RPC = "http://100.113.45.38:8545"
      ROUTER = "0x111111125421ca6dc452d289314280a0f8842a65"
      SELECTOR = "0x07ed2379"
      ROUTER_CLEAN = ROUTER[2:].lower()

      def rpc(method, params):
          req = json.dumps({"jsonrpc":"2.0","id":1,"method":method,"params":params}).encode()
          with urllib.request.urlopen(urllib.request.Request(RPC, req, {"Content-Type":"application/json"})) as r:
              return json.load(r)["result"]

      def get_data_payload(inp):
          bs = bytes.fromhex(inp[2:])
          data_offset = int.from_bytes(bs[4+256 : 4+288], 'big')
          data_start = 4 + data_offset
          data_len = int.from_bytes(bs[data_start:data_start+32], 'big')
          return bs[data_start+32 : data_start+32+data_len]

      def scan_for_fee_recipient(payload):
          for i in range(0, len(payload) - 63):
              if int.from_bytes(payload[i:i+32], 'big') == 20:
                  candidate = payload[i+32:i+52]
                  if candidate != b'\x00'*20 and payload[i+52:i+64] == b'\x00'*12:
                      return i, '0x' + candidate.hex()
          return None, None

      cur = int(rpc("eth_blockNumber", []), 16)
      print(f"Scanning blocks {cur-2000} to {cur} ...")

      checked = 0
      integrators_found = []

      for bn in range(cur, cur-2000, -1):
          block = rpc("eth_getBlockByNumber", [hex(bn), True])
          if not block:
              continue
          for tx in block["transactions"]:
              if (tx.get("to") or "").lower() == ROUTER.lower() and tx["input"].startswith(SELECTOR):
                  checked += 1
                  try:
                      payload = get_data_payload(tx["input"])
                      _, addr = scan_for_fee_recipient(payload)
                      if addr and addr[2:].lower() != ROUTER_CLEAN:
                          integrators_found.append((tx["hash"], addr))
                          print(f"  INTEGRATOR FOUND: {tx['hash']}  feeRecipient={addr}")
                          if len(integrators_found) >= 5:
                              break
                  except:
                      pass
          if len(integrators_found) >= 5:
              break

      print(f"\nChecked {checked} Classic Swap txs, found {len(integrators_found)} integrator txs")
      EOF)