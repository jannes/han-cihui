import json
import pkuseg
from typing import *


def segment(book_json: str):
    seg = pkuseg.pkuseg()
    book = json.loads(book_json)
    title = book['title']
    author = book['author']
    chapters = book['chapters']

    book_cut = seg.cut(title)
    book_cut.extend(seg.cut(author))
    chapters_output = []
    for chapter in chapters:
        chapter_title = chapter['title']
        chapter_content = chapter['content']
        cut = seg.cut(chapter_title)
        cut.extend(seg.cut(chapter_content))
        chapter_output = {'title': chapter_title, 'cut': cut}
        chapters_output.append(chapter_output)

    return {'cut': book_cut, 'chapters': chapters_output}


def segment_dump(book_json: str):
    print(json.dumps(segment(book_json), ensure_ascii=False).encode('utf8').decode())


if __name__ == '__main__':
    # append the call to segment_dump with the inserted json_str here
    pass
