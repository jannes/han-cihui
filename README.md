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
  (only show words that occur at least x times or/and words that contain unknown characters which occur at least x times)
- Create word lists for specific filter of unknown words (e.g all unkown words that occur at least 3 times)
- Filter word lists by chapter to remove words one does not want to study, export per-chapter final word lists

# Demo
![demo-gif](./demo.gif)
