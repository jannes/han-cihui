import sys
import os
import json
sys.path.insert(1, os.path.join(sys.path[0], '../src'))
import pkuseg_segment_book

with open('book.json') as f:
    book = json.load(f)

pkuseg_segment_book.segment_dump(json.dumps(book))
