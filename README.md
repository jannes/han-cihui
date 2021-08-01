# Chinese Vocabulary Manager
A TUI tool for managing my Mandarin vocabulary and analyzing vocabulary in ebooks (epub).  
Requires my [han-segmenter](https://github.com/jannes/han-segmenter) CLI tool for text segmentation.

## Vocabulary Managment
- Manually add known words
- Synchronize with vocabulary in Anki (flashcard software)
- Display statistics about known words/characters, which are being actively studied etc.
- Export known words

## Ebook analysis
- Analyze vocabulary in epub ebook (using my [epubparse](https://github.com/jannes/epubparse) library for parsing)
- Show amount of words/characters known and unknown
- Supports filtering by word's and character's amount of occurrence within text  
  (only show words that occur at least x times or/and words that contain  
   unknown characters which occur at least x times)
- Export filtered unknown vocabulary ordered by chapters as JSON  
  (I use another [tool](https://github.com/jannes/zh-vocab-filter) to create word lists to study from this output)

# Demo
![demo-gif](./demo.gif)