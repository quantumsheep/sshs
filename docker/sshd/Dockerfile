FROM alpine:3.21

RUN apk add --no-cache openssh \
    && ssh-keygen -A \
    && adduser -D -s /bin/sh testuser \
    && passwd -u testuser \
    && mkdir -p /home/testuser/.ssh \
    && chmod 700 /home/testuser/.ssh \
    && chown testuser:testuser /home/testuser/.ssh

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 22

ENTRYPOINT ["/entrypoint.sh"]
CMD ["/usr/sbin/sshd", "-D", "-e"]
