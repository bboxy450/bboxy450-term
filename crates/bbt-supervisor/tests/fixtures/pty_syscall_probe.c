// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Original diagnostic: retain the failed Darwin PTY syscall and device path.
// This probe neither retries a spawn nor identifies issue 38 by a different errno.

#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

enum { WORKERS = 16, ATTEMPTS = 4000, SAMPLES = 20 };
static atomic_uint ready;
static atomic_uint failures;

static void *probe(void *context) {
    const unsigned worker = *(const unsigned *)context;
    atomic_fetch_add(&ready, 1);
    while (atomic_load(&ready) != WORKERS) {
        // Start the independent allocation streams together, at most 16 pairs.
    }
    for (unsigned attempt = 0; attempt < ATTEMPTS; ++attempt) {
        int master = -1, slave = -1;
        char name[128] = {0};
        const char *stage = "posix_openpt", *path = "/dev/ptmx";
        master = posix_openpt(O_RDWR | O_NOCTTY);
        if (master < 0) goto failed;
        stage = "grantpt";
        if (grantpt(master) < 0) goto failed;
        stage = "unlockpt";
        if (unlockpt(master) < 0) goto failed;
        stage = "ptsname_r";
        if (ptsname_r(master, name, sizeof(name)) != 0) goto failed;
        stage = "open_slave";
        path = name;
        slave = open(name, O_RDWR | O_NOCTTY);
        if (slave < 0) goto failed;
        close(slave);
        close(master);
        continue;
    failed: {
        // Save errno before logging/close can change it. No argv or environment.
        const int error = errno;
        if (atomic_fetch_add(&failures, 1) < SAMPLES) {
            fprintf(stderr,
                    "worker=%u attempt=%u stage=%s path=%s errno=%d (%s)\n",
                    worker, attempt, stage, path, error, strerror(error));
        }
        if (slave >= 0) close(slave);
        if (master >= 0) close(master);
    }
    }
    return NULL;
}

int main(void) {
    pthread_t threads[WORKERS];
    unsigned workers[WORKERS];
    for (unsigned worker = 0; worker < WORKERS; ++worker) {
        workers[worker] = worker;
        const int error = pthread_create(&threads[worker], NULL, probe, &workers[worker]);
        if (error != 0) {
            fprintf(stderr, "pthread_create: %s\n", strerror(error));
            return 2;
        }
    }
    for (unsigned worker = 0; worker < WORKERS; ++worker) {
        const int error = pthread_join(threads[worker], NULL);
        if (error != 0) {
            fprintf(stderr, "pthread_join: %s\n", strerror(error));
            return 2;
        }
    }
    const unsigned count = atomic_load(&failures);
    printf("%d native threads x %d attempts; max %d live PTY pairs; failures=%u\n",
           WORKERS, ATTEMPTS, WORKERS, count);
    return count == 0 ? 0 : 1;
}
