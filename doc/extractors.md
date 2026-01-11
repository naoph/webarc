# Extractors
A webarc extractor is a standalone executable located within `$PATH`. Extractors should adhere to the following conventions:

### Parameters
  - Target URL
  - Cookie jar (format TBD)

### Exit status
  - If the extractor determines that intrinsic properties of a URL (protocol, domain, etc) make it not extractable, exit `10`.
  - If the extractor fails to extract an ostensibly-extractable URL, exit `11`.
  - If the extractor succeeds, exit `0`.

### Content output
  - If extraction succeeds, the extracted content is output via stdout.
  - The extracted content takes the form of a gzipped tarball
