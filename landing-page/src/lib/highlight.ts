const MARK = "==";

export interface Part {
  text: string;
  hl: boolean;
}

// Copy marks its one proof as ==words==; odd pieces are the highlighted ones.
export function highlight(text: string): Part[] {
  return text
    .split(MARK)
    .map((piece, index) => ({ text: piece, hl: index % 2 === 1 }))
    .filter((part) => part.text.length > 0);
}
