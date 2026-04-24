from telegram.ext import Application


def main() -> None:
    app = Application.builder().token("TOKEN").build()
    app.run_polling()


if __name__ == "__main__":
    main()
