murmur-xor
==========

Find key neighbors from MurmurHash64A hashes.

Numbers from looking up Darktide file keys:
- ~16k keys and ~210k hashes
- finds ~1.7k new keys
- detects 8 false positives
- takes 0.5 seconds

## Usage

murmur-xor expects keys to only contain `a-z0-9_/`.

murmur-xor requires:
- `KEY_FILE` text file with a key on each line
- `HASH_FILE` text file with a hex encoded hash on each line

Print found keys:
```
murmur-xor --hashes HASH_FILE KEY_FILE
```

Write found keys to `found.txt`:
```
murmur-xor --hashes HASH_FILE KEY_FILE --output found.txt
```

See `murmur-xor --help` for more options.
