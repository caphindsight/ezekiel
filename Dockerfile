FROM caphindsight/mmark:latest
COPY target/release/ezekiel /ezekiel
ENTRYPOINT ["/ezekiel"]
LABEL io.whalebrew.config.keep_container_user 'true'
