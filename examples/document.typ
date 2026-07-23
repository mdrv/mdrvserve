#set page(paper: "a4", margin: (top: 2.2cm, bottom: 2.2cm, x: 2cm), numbering: "1")
#set text(font: ("New Computer Modern", "Noto Sans CJK JP"), size: 11pt)
#set par(justify: true, leading: 0.85em)

#align(center)[
  #text(size: 20pt, weight: "bold")[セグメント木] \
  #text(size: 12pt)[競技プログラミング講座 — 第 1 回]
]
#v(1.2em)

= モチベーション

長さ $n$ の配列に対して、次の 2 種類のクエリを大量に処理したい。

+ 一点更新: 位置 $i$ の値を $x$ に書き換える。
+ 区間クエリ: 区間 $[l, r)$ 上の集約値 (和・積・min など) を求める。

素朴な配列では、一点更新は $O(1)$ だが区間クエリは $O(n)$ になる。
*セグメント木* (Segment Tree) を使うと、両方とも $O(log n)$ で処理できる。

= モノイド

セグメント木が扱える集約は、*モノイド* の条件を満たす必要がある。

$ "結合律:" quad (a ∘ b) ∘ c = a ∘ (b ∘ c) \
  "単位元:" quad exists(e, forall(x, e ∘ x = x ∘ e = x)) $

典型例は $(plus, 0)$、$(times, 1)$、$(min, +infinity)$ など。可換でなくてもよい。

= 計算量

#table(
  columns: 2,
  [*操作*], [*計算量*],
  [構築], $O(n)$,
  [一点更新], $O(log n)$,
  [区間クエリ], $O(log n)$,
)

木の高さは $ceil(log_2 n)$ であり、各クエリはレベルごとに高々 $4$ 個のノードを
参照するため、計算量は $O(log n)$ となる。

= 実装の核

完全二分木を配列に載せる。葉は添字 $N$ から $2 N - 1$ ($N$ は $n$ 以上の最小の
$2$ 冪)、内点 $i$ の子は $2 i$ と $2 i + 1$ である。

```
// Range query on [l, r).   op: monoid binary op,   E: identity element.
fn query(mut l: usize, mut r: usize) -> i64 {
    l += N;
    r += N;
    let (mut vl, mut vr) = (E, E);
    while l < r {
        if l & 1 == 1 {
            vl = op(vl, data[l]);
            l += 1;
        }
        if r & 1 == 1 {
            r -= 1;
            vr = op(data[r], vr);
        }
        l >>= 1;
        r >>= 1;
    }
    op(vl, vr)
}
```

#pagebreak()

= いつセグメント木を選ぶか

- 一点更新と区間クエリの組み合わせが主な用途。
- 演算がモノイド則 (結合律 + 単位元) を満たす。可換でなくてもよい。
- 区間に対する *更新* (区間加算など) が必要なら、*遅延セグメント木* を使う。

= 類似データ構造との比較

#table(
  columns: 3,
  [*構造*], [*更新*], [*クエリ*],
  [素朴配列], $O(1)$, $O(n)$,
  [Binary Indexed Tree], $O(log n)$, $O(log n)$,
  [セグメント木], $O(log n)$, $O(log n)$,
  [遅延セグメント木], $O(log n)$, $O(log n)$,
)

BIT は実装が短いが可換モノイドのみ。セグメント木は非可換にも対応し、
遅延版に拡張すれば区間更新も扱える。

#v(2em)
#align(center)[
  #emph[次回: 遅延セグメント木と区間更新。]
]
