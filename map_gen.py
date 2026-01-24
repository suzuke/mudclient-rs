import re
import sys
import os

def parse_logs(file_paths):
    all_rooms = []
    
    # Regex patterns
    exit_pattern = re.compile(r'^\[出口: (.*)\]$')
    prompt_pattern = re.compile(r'^\(hp\d+/\d+ ma\d+/\d+ (?:mv|v)\d+/\d+ p\d+/\d+.*?\)$')
    
    # Commands that should never be room names
    COMMANDS_TO_IGNORE = {
        'l', 'look', 'n', 's', 'e', 'w', 'u', 'd', 
        'north', 'south', 'east', 'west', 'up', 'down', 
        'i', 'inv', 'inventory', 'stat', 'score', 'wa', 'who',
        'say', 'tell', 'chat', 'y', 'n', 'yes', 'no', 'em'
    }

    all_links = []
    node_map = {}

    print("graph TD")

    for file_path in file_paths:
        if not os.path.exists(file_path):
            continue
            
        with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
            lines = f.readlines()

        rooms = []
        last_end_index = 0
        for i, line in enumerate(lines):
            line = line.strip()
            if not line:
                continue
                
            exit_match = exit_pattern.match(line)
            if exit_match:
                exits_str = exit_match.group(1)
                exits = exits_str.split()
                
                room_name = None
                potential_name_idx = -1
                for j in range(i-1, last_end_index, -1):
                    # ... (same logic as before, but encapsulated here)
                    prev_line = lines[j].strip()
                    if not prev_line: continue
                    if prompt_pattern.match(prev_line):
                        for k in range(j+1, i):
                            n_line = lines[k].strip()
                            if n_line and not n_line.startswith('>>>') and len(n_line) < 30:
                                room_name = n_line
                                break
                        break
                    if not room_name and len(prev_line) < 30 and not prev_line.startswith('(') and not prev_line.startswith('['):
                        potential_name_idx = j

                if not room_name and potential_name_idx != -1:
                    for j in range(last_end_index, i):
                        l = lines[j].strip()
                        if l and not l.startswith('(') and not l.startswith('>') and not l.startswith('>>>'):
                            room_name = l
                            break
                
                if not room_name: room_name = "Unknown"
                rooms.append({'name': room_name, 'exits': exits, 'index': i})
                last_end_index = i

        # Link rooms for this file
        for i in range(len(rooms) - 1):
            room_a, room_b = rooms[i], rooms[i+1]
            move_cmd = None
            for j in range(room_a['index'], room_b['index']):
                cmd_line = lines[j].strip().lower()
                base_cmd = cmd_line.split()[0] if cmd_line else ""
                if base_cmd in ['n', 's', 'e', 'w', 'u', 'd', 'north', 'south', 'east', 'west', 'up', 'down']:
                    move_cmd = base_cmd
                    break
            
            if room_a['name'] in COMMANDS_TO_IGNORE or room_b['name'] in COMMANDS_TO_IGNORE:
                continue

            if move_cmd:
                dir_map = {'n': '北', 's': '南', 'e': '東', 'w': '西', 'u': '上', 'd': '下'}
                dir_zh = dir_map.get(move_cmd, move_cmd)
                all_links.append((room_a['name'], room_b['name'], dir_zh))
            elif room_a['name'] != room_b['name']:
                all_links.append((room_a['name'], room_b['name'], "move"))

        # Add nodes to global map
        for r in rooms:
            name = r['name']
            if name not in COMMANDS_TO_IGNORE and name not in node_map:
                safe_id = f"node_{len(node_map)}"
                print(f'    {safe_id}["{name}"]')
                node_map[name] = safe_id

    # Print links
    seen_links = set()
    for start, end, label in all_links:
        if start in node_map and end in node_map:
            id_start, id_end = node_map[start], node_map[end]
            if (id_start, id_end, label) not in seen_links:
                print(f'    {id_start} -- "{label}" --> {id_end}')
                seen_links.add((id_start, id_end, label))

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python map_gen.py <log_file1> [log_file2 ...]")
    else:
        parse_logs(sys.argv[1:])
