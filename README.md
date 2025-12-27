# GROP

Trigram Accelerated Regex Search

```
$ time grep '"20250522"' ~/.local/share/testdata/testdata.csv
real    0m6.785s

$ time cargo run --release run '"20250522"' ~/.local/share/testdata/testdata.csv
real    0m1.461s
```

IMPORTANT: This project was/is my way of learning Rust and shouldn't be trusted. I don't know what I'm doing.

## Fast

Nothing's free. `grop` needs to build a search index up front:

```
$ time cargo run --release db ~/.local/share/testdata/testdata.csv
                 chunks: 76587
               trigrams: 21911
              file size: 9.36 GiB
                db size:  99.53 MiB (01.04% of file)
      trigrams map size: 116.50 KiB (00.11% of db)
trigrams inventory size:  98.83 MiB (99.30% of db)
        chunk ends size: 299.17 KiB (00.29% of db)
 chunk line counts size: 299.17 KiB (00.29% of db)

real    0m59.517s
```

Once it has the index, `grop` turns your regex into a "query" of "trigrams". It evaluates the queries to make "candidates" and then runs the regex on the candidates to find the matching lines.

### Queries
`grop` uses the lovely `regex-syntax` crate to parse the regex into a `Hir` (High level Intermediate Representation) and then walks that to build a "query" to identify candidate matches. The "query" *must* find all lines that *might* match and *should* skip lines that *can't* match.

The smallest unit of a query is a "trigram". "Trigram" just means a sequence of three things. In `grop`'s case, it's three byes. If `grop` wants to search for the word `piglet`, it instead searches for `pig`, `igl`, `gle`, `let`. To search for `日本`, it searches for `0x97a5e6`, `0xa5e69c`, `0xe697a5`, and `0xe69cac`.

`grop` understands enough about regexes to make `|` into `Or` and sequences in `And`. Examples:

<table>
<tr><th>Regex</th><th>Query</th></tr>

<tr><td><pre>0522</pre></td><td><pre>And[052, 522]</pre></td></tr>

<tr><td><pre>"20250522"</pre></td><td><pre>And["20, 025, 052, 202, 22", 250, 505, 522]</pre></td></tr>

<tr><td><pre>"202..522"</pre></td><td><pre>
And[
    And["20, 202],
    And[22", 522],
] // TODO merge these `And`s
</pre></td></tr>

<tr><td><pre>cat|dog|piglet</pre></td><td><pre>
Or[
    cat,
    dog,
    And[gle, igl, let, pig],
]</pre></td></tr>

<tr><td><pre>(cat)|dog\d|piglet+</pre></td><td><pre>
Or[
    cat,
    dog,
    And[gle, igl, pig],
] // Note the imperfect extractions</pre></td></tr>
</table>

`grop` uses it's search index to evaluate the queries to an ascending sequence of candidates and then uses the `grep` crate on just those candidates. Ascending sequences because:
1. They encode well on disk
2. Are easier to evaluate (AND and OR are streaming merge sorts)
3. Disks like reading in ascending order (much less so with SSDs, but read ahead is still a thing)
4. It's easy to terminate early if we need to.
5. It's a tradition

### Chunks

The `grep` crate is *very* fast. So fast that `grop` doesn't need to operate on individual lines. Instead, it indexes "chunks" of lines. But default it breaks out a new chunk on the next line break after 128kb. So "canidates" aren't lines. They are 128kb "chunks".

This massively decreases the size of the search index at the price of massive increases the number of false positive candidates. But, again, `grep` is really fast. And modern disks are fast. In other contexts this tradeoff won't make sense.

### Query pruning

After converting the regex into a query tree, `grop` rewrites that tree against the search index, sprinkling in enough data so we can actually execute it. While it's there, it converts missing trigrams into `MatchNone` queries which it flows around the `And` and `Or` tree.

`grop` should have enough information to make `MatchAll`. We could also store things like "matches 80% of the chunks" which could reduce the storage size or allow us to skip queries unlikely to filter. But queries are quite fast now.


### Encoding

`grop`'s db is mostly made up of lists of chunks for each trigram so it tries to compress them somewhat. Each chunk id is a `u32`, but `grop` writes deltas with variable-width integers. This looks like:

|      Input      |      Delta      |    Variable    |
| --------------- | --------------- | -------------- |
| `0, 1, 2, 3, 5` | `0, 0, 0, 0, 1` | `0x0000000001` |
|  `0, 129, 130`  |   `0, 128, 0`   |  `0x00a00101`  |

There's a lot more work to do here - some sequences are super dense and we'd be better off encoding what isn't in them. Or just using roaring bitmaps directly. Or something else. A place for further fun.

## Prior art

All of this is pretty traditional to be honest. If `grop` innovates at anything, it's using chunks and throwing them into the already fast `grep` crate.

Look at the Wikipedia page for [Trigram search](https://en.wikipedia.org/wiki/Trigram_search) for some examples. The reference for [Google Code](https://swtch.com/~rsc/regexp/regexp4.html) talks about Shannon using n-grams in 1948 to analyze text. It also references a book called "Managing Gigabytes" which, I expect, uses very similar techniques.

## Simplifying assumptions

* Trigrams are all three bytes - unicode characters *work*, but `grop` just works through them byte by byte.
* If you modify the file, we'll never know. `grop` could detect mtime changes and rebuild the index or patch it. But that's a problem for later.
* `grop` assumes that input files are never more then `u32::MAX` chunks.