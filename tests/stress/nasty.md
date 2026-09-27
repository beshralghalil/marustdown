# Hostile input

## Terminal escape injection (must render as harmless replacement characters)

Red text attempt: [31mRED[0m, clear screen [2J, cursor move [10;10H.

Clipboard write attempt: ]52;c;cGF3bmVk and title change ]0;owned.

Bell , backspace , carriage return in the middle, form feed , NUL-ish .

C1 control CSI 31m and OSC 0;x.

[link with escape in url](https://example.com/]52;c;eA==)

```sh
echo '[31mred inside code[0m'
```

## Very long word

AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA

## Deep nesting

> quote level 1
> > quote level 2
> > > quote level 3
> > > > quote level 4
> > > > > quote level 5
> > > > > > quote level 6
> > > > > > > quote level 7
> > > > > > > > quote level 8
> > > > > > > > > quote level 9
> > > > > > > > > > quote level 10
> > > > > > > > > > > quote level 11
> > > > > > > > > > > > quote level 12
> > > > > > > > > > > > > quote level 13
> > > > > > > > > > > > > > quote level 14
> > > > > > > > > > > > > > > quote level 15
> > > > > > > > > > > > > > > > quote level 16
> > > > > > > > > > > > > > > > > quote level 17
> > > > > > > > > > > > > > > > > > quote level 18
> > > > > > > > > > > > > > > > > > > quote level 19
> > > > > > > > > > > > > > > > > > > > quote level 20
> > > > > > > > > > > > > > > > > > > > > quote level 21
> > > > > > > > > > > > > > > > > > > > > > quote level 22
> > > > > > > > > > > > > > > > > > > > > > > quote level 23
> > > > > > > > > > > > > > > > > > > > > > > > quote level 24
> > > > > > > > > > > > > > > > > > > > > > > > > quote level 25
> > > > > > > > > > > > > > > > > > > > > > > > > > quote level 26
> > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 27
> > > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 28
> > > > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 29
> > > > > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 30
> > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 31
> > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 32
> > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 33
> > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 34
> > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 35
> > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 36
> > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 37
> > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 38
> > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 39
> > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > > quote level 40

- list level 0
  - list level 1
    - list level 2
      - list level 3
        - list level 4
          - list level 5
            - list level 6
              - list level 7
                - list level 8
                  - list level 9
                    - list level 10
                      - list level 11
                        - list level 12
                          - list level 13
                            - list level 14
                              - list level 15
                                - list level 16
                                  - list level 17
                                    - list level 18
                                      - list level 19
                                        - list level 20
                                          - list level 21
                                            - list level 22
                                              - list level 23
                                                - list level 24
                                                  - list level 25
                                                    - list level 26
                                                      - list level 27
                                                        - list level 28
                                                          - list level 29

## Unicode edge cases

Zero width​space, ZWJ family 👨‍👩‍👧‍👦, flags 🇸🇾🇩🇪, combining é vs é,
RTL: مرحبا بالعالم and שלום עולם, fullwidth ＡＢＣ１２３, math 𝕏𝔸𝕄, box ─│┌┐.

## Many links (hint tags go to two letters)

[link 0](https://example.com/0) [link 1](https://example.com/1) [link 2](https://example.com/2) [link 3](https://example.com/3) [link 4](https://example.com/4) [link 5](https://example.com/5) [link 6](https://example.com/6) [link 7](https://example.com/7) [link 8](https://example.com/8) [link 9](https://example.com/9) [link 10](https://example.com/10) [link 11](https://example.com/11) [link 12](https://example.com/12) [link 13](https://example.com/13) [link 14](https://example.com/14) [link 15](https://example.com/15) [link 16](https://example.com/16) [link 17](https://example.com/17) [link 18](https://example.com/18) [link 19](https://example.com/19) [link 20](https://example.com/20) [link 21](https://example.com/21) [link 22](https://example.com/22) [link 23](https://example.com/23) [link 24](https://example.com/24) [link 25](https://example.com/25) [link 26](https://example.com/26) [link 27](https://example.com/27) [link 28](https://example.com/28) [link 29](https://example.com/29) [link 30](https://example.com/30) [link 31](https://example.com/31) [link 32](https://example.com/32) [link 33](https://example.com/33) [link 34](https://example.com/34) [link 35](https://example.com/35) [link 36](https://example.com/36) [link 37](https://example.com/37) [link 38](https://example.com/38) [link 39](https://example.com/39) [link 40](https://example.com/40) [link 41](https://example.com/41) [link 42](https://example.com/42) [link 43](https://example.com/43) [link 44](https://example.com/44) [link 45](https://example.com/45) [link 46](https://example.com/46) [link 47](https://example.com/47) [link 48](https://example.com/48) [link 49](https://example.com/49) [link 50](https://example.com/50) [link 51](https://example.com/51) [link 52](https://example.com/52) [link 53](https://example.com/53) [link 54](https://example.com/54) [link 55](https://example.com/55) [link 56](https://example.com/56) [link 57](https://example.com/57) [link 58](https://example.com/58) [link 59](https://example.com/59)

## Odd tables

| a | b | c |
|---|---|---|
| only one cell |
| 1 | 2 | 3 | 4 | 5 |
|  |  |  |

| x |
|---|

## Empty and odd headings

#

##

###### deep

# `code only`

# **bold** *em* ~~strike~~

Setext
===

## HTML

<div>
<script>alert(1)</script>
</div>

<img src=x onerror=alert(1)>

## Footnotes (unsupported)

Text[^1].

[^1]: note

## Unclosed fence at the end of the file

```rust
fn never_closed() {
