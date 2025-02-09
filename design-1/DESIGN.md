# What kind of syntax transforms would be useful?

E.g. some global ones like let, while, etc. Could these be lazy functions? I don't think so, more like macros

we could add some times like lazy-thunk, symbol and then probably most of these could be represented by lazy functions.

also optimizations like merging consecutive (print) could be done using this?

Or like Scala macros where the input needs to be valid syntax?

https://en.wikipedia.org/wiki/Hygienic_macro

static typing should not be builtin but built on top in the language itself

every "function" gets a list of tokens? / list of lists?

there is a builtin eval function to actually execute code?

lexical environment of stuff is pretty important.

maybe eval-in-scope function