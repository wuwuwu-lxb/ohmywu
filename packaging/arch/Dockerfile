FROM archlinux:base-devel

RUN pacman -Syu --noconfirm --needed \
  base-devel \
  nodejs \
  npm \
  rust \
  gtk3 \
  glib2 \
  gcc-libs \
  hicolor-icon-theme \
  file

RUN useradd -m -u 1000 builder

USER builder
WORKDIR /workspace/packaging/arch

CMD ["makepkg", "--nodeps", "--force"]
