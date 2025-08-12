#/usr/bin/env python3

import json
import shutil

SOURCE_JSON="DisplayItemExcelConfigData.json"
TEXTMAP_JSON="TextMapEN.json"

def get_name_hashes(file_path):
    with open(file_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    return {str(item["nameTextMapHash"]) for item in data if "nameTextMapHash" in item}

possible_hashes = get_name_hashes(SOURCE_JSON)

shutil.copy(TEXTMAP_JSON, TEXTMAP_JSON + ".bak")

with open(TEXTMAP_JSON, "r", encoding="utf-8") as f:
    c_data = json.load(f)

filtered_json = {k: v for k, v in c_data.items() if k in possible_hashes}

with open(TEXTMAP_JSON, "w", encoding="utf-8") as f:
    json.dump(filtered_json, f, ensure_ascii=False, indent=2)

print(f"Filtered {TEXTMAP_JSON}. Kept {len(filtered_json)} entries.")
