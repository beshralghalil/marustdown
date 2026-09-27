# Horizontal scrolling

Put the cursor on a block and press `l` / `h` (or → / ←); `L` / `H` jump to the ends.
Text around the blocks must not move.

## Long code line among short ones

```rust
fn short() {}
fn main() { let s = "this line is much longer than any sane content width this line is much longer than any sane content width this line is much longer than any sane content width this line is much longer than any sane content width "; println!("{s}"); }
fn also_short() {}
```

## Wide characters at the cut edges

Scroll one step at a time: columns must stay aligned, CJK and emoji cut in half become spaces.

```text
日本語のテキスト日本語のテキスト日本語のテキスト日本語のテキスト日本語のテキスト日本語のテキスト
🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡
mixed ascii 日本 ascii 🚀 ascii 日本 ascii 🚀 ascii 日本 ascii 🚀 ascii 日本 ascii 🚀 ascii 日本 ascii 🚀 end
```

## Tabs

```go
func main() {
	if true {
		fmt.Println("this line is much longer than any sane content width this line is much longer than any sane content width this line is much longer than any sane content width this line is much longer than any sane content width ")
	}
}
```

## One line past the 4096-column cap

```text
xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxEND-SHOULD-NOT-BE-VISIBLE
```

## Minified JSON

```json
{"name":"marustdown","features":["pager","search","tasks","math","diagrams","links","outline","edit"],"nested":{"a":{"b":{"c":{"d":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20]}}}},"long":"this line is much longer than any sane content width this line is much longer than any sane content width this line is much longer than any sane content width this line is much longer than any sane content width "}
```

## Three-digit line numbers, long line deep inside

```python
x_1 = 1
x_2 = 2
x_3 = 3
x_4 = 4
x_5 = 5
x_6 = 6
x_7 = 7
x_8 = 8
x_9 = 9
x_10 = 10
x_11 = 11
x_12 = 12
x_13 = 13
x_14 = 14
x_15 = 15
x_16 = 16
x_17 = 17
x_18 = 18
x_19 = 19
x_20 = 20
x_21 = 21
x_22 = 22
x_23 = 23
x_24 = 24
x_25 = 25
x_26 = 26
x_27 = 27
x_28 = 28
x_29 = 29
x_30 = 30
x_31 = 31
x_32 = 32
x_33 = 33
x_34 = 34
x_35 = 35
x_36 = 36
x_37 = 37
x_38 = 38
x_39 = 39
x_40 = 40
x_41 = 41
x_42 = 42
x_43 = 43
x_44 = 44
x_45 = 45
x_46 = 46
x_47 = 47
x_48 = 48
x_49 = 49
x_50 = 50
x_51 = 51
x_52 = 52
x_53 = 53
x_54 = 54
x_55 = 55
x_56 = 56
x_57 = 57
x_58 = 58
x_59 = 59
x_60 = 60
x_61 = 61
x_62 = 62
x_63 = 63
x_64 = 64
x_65 = 65
x_66 = 66
x_67 = 67
x_68 = 68
x_69 = 69
x_70 = 70
x_71 = 71
x_72 = 72
x_73 = 73
x_74 = 74
x_75 = 75
x_76 = 76
line_77 = 'this line is much longer than any sane content width this line is much longer than any sane content width this line is much longer than any sane content width this line is much longer than any sane content width '
x_78 = 78
x_79 = 79
x_80 = 80
x_81 = 81
x_82 = 82
x_83 = 83
x_84 = 84
x_85 = 85
x_86 = 86
x_87 = 87
x_88 = 88
x_89 = 89
x_90 = 90
x_91 = 91
x_92 = 92
x_93 = 93
x_94 = 94
x_95 = 95
x_96 = 96
x_97 = 97
x_98 = 98
x_99 = 99
x_100 = 100
x_101 = 101
x_102 = 102
x_103 = 103
x_104 = 104
x_105 = 105
x_106 = 106
x_107 = 107
x_108 = 108
x_109 = 109
x_110 = 110
x_111 = 111
x_112 = 112
x_113 = 113
x_114 = 114
x_115 = 115
x_116 = 116
x_117 = 117
x_118 = 118
x_119 = 119
x_120 = 120
x_121 = 121
x_122 = 122
x_123 = 123
x_124 = 124
x_125 = 125
x_126 = 126
x_127 = 127
x_128 = 128
x_129 = 129
x_130 = 130
```

## Wide display math

$$
\sum_{i=1}^{n} a_i + \sum_{j=1}^{m} b_j + \sum_{k=1}^{p} c_k + \int_0^\infty f(x)\,dx + \int_0^\infty g(x)\,dx + \int_0^\infty h(x)\,dx + \alpha + \beta + \gamma + \delta
$$

$$
M = \begin{pmatrix} 1 & 2 & 3 & 4 & 5 & 6 & 7 & 8 & 9 & 10 & 11 & 12 & 13 & 14 & 15 & 16 & 17 & 18 & 19 & 20 \\ 21 & 22 & 23 & 24 & 25 & 26 & 27 & 28 & 29 & 30 & 31 & 32 & 33 & 34 & 35 & 36 & 37 & 38 & 39 & 40 \end{pmatrix}
$$

## Wide diagram

```mermaid
graph LR
    A[Collect requirements] --> B[Design the architecture] --> C[Implement features] --> D[Write tests] --> E[Review code] --> F[Release] --> G[Monitor production]
```

## Wide code inside a quote and a list

> The quote bar must stay while the code scrolls:
>
> ```sh
> echo "this line is much longer than any sane content width this line is much longer than any sane content width this line is much longer than any sane content width this line is much longer than any sane content width "
> ```

- A list item with a wide block:

  ```sh
  curl --proto '=https' --tlsv1.2 -LsSf https://example.com/some/very/long/path/that/keeps/going/and/going/installer.sh | sh -s -- --yes --verbose
  ```

## A table with many columns (wraps, doesn't scroll)

| c0 | c1 | c2 | c3 | c4 | c5 | c6 | c7 | c8 | c9 | c10 | c11 | c12 | c13 | c14 | c15 | c16 | c17 | c18 | c19 | c20 | c21 | c22 | c23 | c24 | c25 | c26 | c27 | c28 | c29 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| value 0 | value 1 | value 2 | value 3 | value 4 | value 5 | value 6 | value 7 | value 8 | value 9 | value 10 | value 11 | value 12 | value 13 | value 14 | value 15 | value 16 | value 17 | value 18 | value 19 | value 20 | value 21 | value 22 | value 23 | value 24 | value 25 | value 26 | value 27 | value 28 | value 29 |

Last paragraph.
