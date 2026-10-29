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
enum stage { OPENPT, GRANT, UNLOCK, NAME, SLAVE, STAGES };
enum error_kind { MISSING, NEGATIVE_SIX, OTHER, ERROR_KINDS };
static const char *const stage_names[STAGES] = {
    "posix_openpt", "grantpt", "unlockpt", "ptsname_r", "open_slave"
};
static atomic_uint ready;
static atomic_uint failures;
static atomic_uint counts[STAGES][ERROR_KINDS];

static void record_failure(enum stage stage, const char *path, int error,
                           unsigned worker, unsigned attempt, FILE *samples) {
    const enum error_kind kind = error == ENOENT ? MISSING
                                : error == -6 ? NEGATIVE_SIX : OTHER;
    const unsigned previous = atomic_fetch_add(&counts[stage][kind], 1);
    const unsigned index = atomic_fetch_add(&failures, 1);
    // Retain every failure in bounded counters. An ENOENT after the general
    // sample cap still keeps its first exact path for each of the five stages.
    if (index < SAMPLES || (kind == MISSING && previous == 0)) {
        fprintf(samples,
                "worker=%u attempt=%u stage=%s path=%s errno=%d (%s)\n",
                worker, attempt, stage_names[stage], path, error, strerror(error));
    }
}

static int check_accounting(void) {
    FILE *samples = tmpfile();
    if (samples == NULL) return 2;
    for (unsigned i = 0; i < SAMPLES; ++i)
        record_failure(OPENPT, "/dev/ptmx", -6, 0, i, samples);
    record_failure(SLAVE, "/dev/ttys-counter-witness", ENOENT, 0, SAMPLES, samples);
    record_failure(GRANT, "/dev/ptmx", EACCES, 0, SAMPLES + 1, samples);
    record_failure(SLAVE, "/dev/ttys-counter-witness", ENOENT, 0, SAMPLES + 2, samples);
    rewind(samples);
    unsigned lines = 0, witnesses = 0;
    char line[512];
    while (fgets(line, sizeof(line), samples) != NULL) {
        ++lines;
        if (strstr(line, "path=/dev/ttys-counter-witness errno=2 ") != NULL)
            ++witnesses;
    }
    fclose(samples);
    if (atomic_load(&failures) != SAMPLES + 3
        || atomic_load(&counts[OPENPT][NEGATIVE_SIX]) != SAMPLES
        || atomic_load(&counts[SLAVE][MISSING]) != 2
        || atomic_load(&counts[GRANT][OTHER]) != 1
        || lines != SAMPLES + 1 || witnesses != 1) {
        fputs("diagnostic accounting or post-cap ENOENT witness lost\n", stderr);
        return 1;
    }
    puts("diagnostic accounting: 23 failures classified; post-cap ENOENT path retained");
    return 0;
}

static void *probe(void *context) {
    const unsigned worker = *(const unsigned *)context;
    atomic_fetch_add(&ready, 1);
    while (atomic_load(&ready) != WORKERS) {
        // Start the independent allocation streams together, at most 16 pairs.
    }
    for (unsigned attempt = 0; attempt < ATTEMPTS; ++attempt) {
        int master = -1, slave = -1;
        char name[128] = {0};
        enum stage stage = OPENPT;
        const char *path = "/dev/ptmx";
        master = posix_openpt(O_RDWR | O_NOCTTY);
        if (master < 0) goto failed;
        stage = GRANT;
        if (grantpt(master) < 0) goto failed;
        stage = UNLOCK;
        if (unlockpt(master) < 0) goto failed;
        stage = NAME;
        if (ptsname_r(master, name, sizeof(name)) != 0) goto failed;
        stage = SLAVE;
        path = name;
        slave = open(name, O_RDWR | O_NOCTTY);
        if (slave < 0) goto failed;
        close(slave);
        close(master);
        continue;
    failed: {
        // Save errno before logging/close can change it. No argv or environment.
        const int error = errno;
        record_failure(stage, path, error, worker, attempt, stderr);
        if (slave >= 0) close(slave);
        if (master >= 0) close(master);
    }
    }
    return NULL;
}

int main(int argc, char **argv) {
    if (argc == 2 && strcmp(argv[1], "--check-accounting") == 0)
        return check_accounting();
    if (argc != 1) return 2;
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
    for (unsigned stage = 0; stage < STAGES; ++stage) {
        printf("stage=%s enoent=%u negative_errno6=%u other=%u\n", stage_names[stage],
               atomic_load(&counts[stage][MISSING]),
               atomic_load(&counts[stage][NEGATIVE_SIX]),
               atomic_load(&counts[stage][OTHER]));
    }
    return count == 0 ? 0 : 1;
}
