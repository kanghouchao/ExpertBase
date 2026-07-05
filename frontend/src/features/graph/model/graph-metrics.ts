// 後端の orphans と同じく、入出力リンクが一つもないノードだけを孤立とする。
export function isOrphanDegree(degree: number): boolean {
  return degree === 0;
}
