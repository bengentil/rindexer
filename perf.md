# Execution speed

Speed observed on a single-core ARM7 with 1.4GiB of data

### rust md-5.Md5, BUFFER_SIZE=4096

real	6m7.157s
user	6m1.820s
sys	0m4.110s

### rust md-5.Md5, BUFFER_SIZE=1024

real	6m14.614s
user	6m6.980s
sys	0m7.130s

### rust sha2.Sha256, BUFFER_SIZE=4096

real	15m40.432s
user	15m34.710s
sys	0m4.790s

### rust sha2.Sha256, BUFFER_SIZE=4M

real	15m34.278s
user	15m17.100s
sys	0m3.030s

### external md5sum *

real	0m16.766s
user	0m10.700s
sys	0m2.780s

### external sha256sum *

real	0m28.833s
user	0m23.530s
sys	0m4.550s

### rust Command sha256sum

real	0m29.561s
user	0m25.130s
sys	0m2.910s

### rust Command md5sum

real	0m17.346s
user	0m10.500s
sys	0m2.960s